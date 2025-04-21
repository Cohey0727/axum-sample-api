use std::collections::HashMap;

use rand::Rng;
use mysql::*;
use mysql::prelude::*;
use chrono::{Duration, NaiveDate, Utc};
use uuid::Uuid;

use crate::config;


pub async fn generate_customers(count: usize) -> Result<()> {
    println!("{}件のユーザーデータを生成します", count);
    
    // データベース接続設定
    let database_url = config::database::get_database_url();
    let opts = mysql::Opts::from_url(&database_url).expect("不正なデータベースURL");
    // Optsオブジェクトを使ってプールを作成
    let pool = mysql::Pool::new(opts).expect("データベース接続に失敗しました");

    
    // 固定のパスワードハッシュ
    let password_hash = "$2y$10$Ik1i3qUFpBICQyg1ZVGZ3eRB.YiPt7oXsfS7ckNhPDwFMkl4BIDbC";
    
    // 固定値
    let shipping_address = "1-12-123";
    let shipping_phone = "03-1234-5678";
    
    // MySQLはasyncに対応していないため、tokioのブロッキング実行を使用
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get_conn()?;
        let mut tx = conn.start_transaction(TxOpts::default())?;
        
        for i in 0..count {
            // 進捗表示（10,000件ごと）
            if i % 10000 == 0 && i > 0 {
                println!("{}/{}件 生成完了", i, count);
            }
            
            let seq_num = i + 1;
            let id = format!("00000000-0000-4000-0000-{:012}", seq_num);
            let email = format!("{}@example.com", id);
            let is_infomercial: u8 = rand::rng().random_range(0..=1);
            let accepts_marketing: u8 = rand::rng().random_range(0..=1);
            let province_num = rand::rng().random_range(1..=47);
            let shipping_province_code = format!("JP-{:02}", province_num);
            
            // 日本の名前をランダムに生成
            let first_names = ["太郎", "次郎", "三郎", "四郎", "五郎", "花子", "梅子", "桃子", "和子", "幸子"];
            let last_names = ["佐藤", "鈴木", "高橋", "田中", "伊藤", "渡辺", "山本", "中村", "小林", "加藤"];
            
            let first_name = first_names[rand::rng().random_range(0..first_names.len())];
            let last_name = last_names[rand::rng().random_range(0..last_names.len())];
            
            // 作成日時と更新日時
            let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            
            // SQLクエリを実行
            tx.exec_drop(
                "INSERT INTO customers (id, email, is_infomercial, password, accepts_marketing, 
                first_name, last_name, shipping_province_code, shipping_address_line1, shipping_phone, created_at, updated_at) 
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (&id, &email, is_infomercial, password_hash, accepts_marketing, 
                 first_name, last_name, shipping_province_code, shipping_address, shipping_phone, &now, &now),
            )?;
        }
        
        tx.commit()?;
        println!("ユーザーデータの生成が完了しました");
        
        Ok::<(), mysql::Error>(())
    })
    .await
    .expect("ブロッキングタスクの実行に失敗")?;
    
    Ok(())
}


pub async fn generate_orders(count: usize) -> Result<()> {
    println!("{}件の注文データを生成します", count);
    
    // データベース接続設定
    let database_url = config::database::get_database_url();
    let opts = mysql::Opts::from_url(&database_url).expect("不正なデータベースURL");
    // Optsオブジェクトを使ってプールを作成
    let pool = mysql::Pool::new(opts).expect("データベース接続に失敗しました");
    
    // MySQLはasyncに対応していないため、tokioのブロッキング実行を使用
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get_conn()?;
        
        // 顧客IDを取得
        println!("顧客データを取得中...");
        let customer_ids: Vec<String> = conn.query("SELECT id, email from customers where id like '00000%'")?
        .into_iter()
        .map(|row| {
            // Row型から(String, String)型にマッピング
            let (id, _email): (String, String) = mysql::from_row(row);
            id  // idだけを返す
        })
        .collect();
        
        println!("{}件の顧客データを取得しました", customer_ids.len());
        
        // 商品情報を取得
        println!("商品データを取得中...");
        let products: Vec<(String, String)> = conn.query("SELECT id, variant_id FROM products WHERE is_suspension = false")?
            .into_iter()
            .map(|row| {
                let (id, variant_id): (String, String) = mysql::from_row(row);
                (id, variant_id)
            })
            .collect();
        
        println!("{}件の商品データを取得しました", products.len());
        
        // トランザクション開始
        let mut tx = conn.start_transaction(TxOpts::default())?;
        
        // 2020年1月1日から現在までの期間を設定
        let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let end_date = Utc::now().naive_utc();
        let date_range = (end_date - start_date).num_seconds() as u64;
        
        let mut order_ids = Vec::with_capacity(count);
        
        for i in 0..count {
            // 進捗表示（10,000件ごと）
            if i % 10000 == 0 && i > 0 {
                println!("{}/{}件 注文データ生成完了", i, count);
            }
            
            // ランダムな顧客を選択
            let customer_index = rand::rng().random_range(0..customer_ids.len());
            let customer_id = &customer_ids[customer_index];
            
            // ランダムな日付を生成（2020年から現在まで）
            let random_seconds = rand::rng().random_range(0..date_range);
            let created_at = start_date + Duration::seconds(random_seconds as i64);

            
            // 配送日は注文日から1週間後
            let delivery_date = (created_at.date() + Duration::days(7)).format("%Y-%m-%d").to_string();
            
            // 注文IDを生成
            let order_id = Uuid::new_v4().to_string();
            order_ids.push(order_id.clone());
            
            // メールアドレスを取得（顧客IDに紐づく）
            let email = format!("{}@example.com", customer_id); // 簡易的に生成
            
            // 配送先住所情報
            let shipping_address = format!(
                r#"{{"zip": "100-0001", "city": "千代田区", "phone": "09012345678", "province": "JP-13", "last_name": "テスト", "first_name": "ユーザー", "address_line1": "1-1-1", "address_line2": "テスト住所", "converted_province": "東京都"}}"#
            );
            
            // 支払い方法は固定で"credit"
            let payment_method = "credit";
            
            // 配送温度は固定で"Normal"
            let shipping_temperature = "Normal";
            
            // 定期購入はfalse
            let note = "";
            
            // 定期購入関連の値も0に設定
            let subscription_discount_rate = 0;
            let discount_plan_name = "";
            let discount_plan_rate = 0;
            
            // 日時フォーマット
            let created_at_str = created_at.format("%Y-%m-%d %H:%M:%S").to_string();
            
            // SQLクエリを実行
            let query = format!(
                "INSERT INTO orders (id, email, customer_id, delivery_date, delivery_timezone, note, 
                payment_method, total_price, subtotal_price, total_tax, currency, total_line_items_price, 
                total_discounts, shipping_address, financial_status, fulfillment_status, 
                processed_at, created_at, updated_at, point_discount, coupon_discount, 
                subscription_discount_rate, discount_plan_name, discount_plan_rate, shipping_temperature, 
                is_non_face_to_face_receipt, paid_points_discount, free_points_discount, is_fast_delivery, 
                delivery_location_code) 
                VALUES ('{}', '{}', '{}', '{}', 'free', '{}', 
                '{}', 0, 0, 0, 'jpy', 0, 
                0, '{}', 'paid', 'null', 
                '{}', '{}', '{}', 0, 0, 
                {}, '{}', {}, '{}', 
                0, 0, 0, 0, 
                '00')",
                order_id, email, customer_id, delivery_date, note,
                payment_method, shipping_address,
                created_at_str, created_at_str, created_at_str,
                subscription_discount_rate, discount_plan_name, discount_plan_rate, shipping_temperature
            );            
            tx.exec_drop(
                query,
                (),
            )?;
        }
        
        println!("注文データの生成が完了しました。注文商品データを生成します...");
        
        // 注文商品データを生成
        generate_order_products(&mut tx, &order_ids, &products)?;
        
        tx.commit()?;
        println!("注文データと注文商品データの生成が完了しました");
        
        Ok::<(), mysql::Error>(())
    })
    .await
    .expect("ブロッキングタスクの実行に失敗")?;
    
    Ok(())
}

fn generate_order_products(tx: &mut Transaction, order_ids: &[String], products: &[(String, String)]) -> Result<(), mysql::Error> {
    for (i, order_id) in order_ids.iter().enumerate() {
        // 進捗表示（10,000件ごと）
        if i % 10000 == 0 && i > 0 {
            println!("{}/{}件 注文商品データ生成完了", i, order_ids.len());
        }
        
        // 各注文に1〜5個の商品を追加
        let product_count = rand::rng().random_range(1..=5);
        
        for _ in 0..product_count {
            // ランダムな商品を選択
            let product_index = rand::rng().random_range(0..products.len());
            let (product_id, variant_id) = &products[product_index];
            
            // 数量をランダムに決定
            let quantity = rand::rng().random_range(1..=3);
            
            // 価格は0円
            let price = 0;
            
            // 定期購入はfalse
            let is_subscription = 0;
            
            // 新規割引かどうかをランダムに決定
            let is_brand_new_discount = 0;
            
            // 現在の日時
            let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            
            
            // SQLクエリを実行（format!マクロを使用）
            let query = format!(
                "INSERT INTO order_products (order_id, product_id, variant_id, quantity, price, 
                is_subscription, is_brand_new_discount, created_at, updated_at) 
                VALUES ('{}', '{}', '{}', {}, {}, 
                {}, {}, '{}', '{}')",
                order_id, product_id, variant_id, quantity, price,
                is_subscription, is_brand_new_discount, now, now
            );

            tx.exec_drop(query, ())?;
        }
    }
    
    Ok(())
}