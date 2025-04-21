use mysql::prelude::Queryable;
use std::{collections::HashMap, sync::Arc};

// 商品IDとインデックスのマッピングを保持する構造体
pub struct ProductDimensions {
    product_to_index: HashMap<String, usize>,
    dimension: usize,
}

impl ProductDimensions {
    // 新しいインスタンスを作成
    pub fn new(product_ids: Vec<String>) -> Self {
        let mut product_to_index = HashMap::new();

        // 各商品IDにインデックスを割り当て
        for (idx, product_id) in product_ids.into_iter().enumerate() {
            product_to_index.insert(product_id, idx);
        }

        // 次元数を先に計算して保存
        let dimension = product_to_index.len();

        ProductDimensions {
            product_to_index,
            dimension,
        }
    }

    // 商品IDからインデックスを取得
    pub fn get_index(&self, product_id: &str) -> Option<usize> {
        self.product_to_index.get(product_id).copied()
    }

    // ベクトルの次元数を取得
    pub fn get_dimension(&self) -> usize {
        self.dimension
    }
}

// ユーザーベクトル表現のための構造体
pub struct UserVector {
    pub region_vector: Vec<f32>,
    pub product_vector: Vec<f32>,
}

// 地域コードをベクトルに変換する関数
pub fn region_to_vector(province_code: &str) -> Vec<f32> {
    // JP-XX 形式から数値部分を抽出
    let region_value = if province_code.starts_with("JP-") && province_code.len() >= 5 {
        // 数値部分を抽出して整数に変換
        province_code[3..].parse::<u32>().unwrap_or(0)
    } else {
        // 不正な形式の場合はデフォルト値
        0
    };

    // 都道府県コードを1〜47の範囲で正規化
    let normalized_value = if region_value >= 1 && region_value <= 47 {
        region_value as f32 / 47.0
    } else {
        0.0
    };

    vec![normalized_value]
}

// 商品情報を表す汎用的な構造体
pub struct ProductItem {
    pub product_id: String,
    pub quantity: u32,
}

// カート内商品をベクトルに変換する関数
pub fn products_to_vector(
    products: &[ProductItem],
    product_dimensions: &ProductDimensions,
) -> Vec<f32> {
    let dimension = product_dimensions.get_dimension();
    let mut vector = vec![0.0; dimension];

    for product in products {
        // 商品IDに対応するインデックスを取得
        if let Some(index) = product_dimensions.get_index(&product.product_id) {
            // 数量を対応する次元に設定
            vector[index] = product.quantity as f32;
        }
    }

    // ベクトルの正規化（オプション）
    let magnitude = vector.iter().map(|&x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for val in &mut vector {
            *val /= magnitude;
        }
    }

    vector
}

// ユーザーベクトルを作成する関数
pub fn create_user_vector(
    region_code: &str,
    products: &[ProductItem],
    product_dimensions: &ProductDimensions,
) -> UserVector {
    UserVector {
        region_vector: region_to_vector(region_code),
        product_vector: products_to_vector(products, product_dimensions),
    }
}

// データベースから有効な商品IDのリストを取得
pub async fn fetch_product_dimensions(
    pool: &mysql::Pool,
) -> Result<ProductDimensions, mysql::Error> {
    let mut conn = pool.get_conn()?;

    // 有効な商品IDを取得するクエリ
    let product_ids: Vec<String> = conn.query_map(
        "SELECT variant_id FROM products WHERE is_suspension = false",
        |variant_id: String| variant_id,
    )?;

    Ok(ProductDimensions::new(product_ids))
}

pub async fn get_similar_products(
    pool: &Arc<mysql::Pool>,
    current_user: &UserVector,
    current_products: &[ProductItem],
    product_dimensions: &ProductDimensions,
) -> Vec<(String, f32)> {
    // 汎用的な(製品ID, スコア)のタプルを返す
    // 現在のカートに含まれる商品IDのセットを作成
    let current_product_ids: std::collections::HashSet<String> = current_products
        .iter()
        .map(|p| p.product_id.clone())
        .collect();

    // 他のユーザーの購入履歴を取得
    let other_users = match fetch_user_purchase_history(pool, product_dimensions).await {
        Ok(users) => users,
        Err(_) => return vec![], // エラー処理を簡略化
    };

    // 類似度計算と上位ユーザー抽出
    let mut user_similarities: Vec<(usize, f32)> = other_users
        .iter()
        .enumerate()
        .map(|(idx, user)| (idx, combined_similarity(current_user, user, 0.2)))
        .collect();

    // 類似度で降順ソート
    user_similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 上位N人のユーザーを抽出
    const TOP_USERS: usize = 10;
    let top_users: Vec<usize> = user_similarities
        .iter()
        .take(TOP_USERS)
        .map(|(idx, _)| *idx)
        .collect();

    // 商品スコアを集計
    let mut product_scores: HashMap<String, f32> = HashMap::new();

    for &user_idx in &top_users {
        if user_idx < other_users.len() {
            // 上位ユーザーの購入履歴からスコアを集計
            let user_products = fetch_user_products(pool, user_idx as u64)
                .await
                .unwrap_or_default();

            for product in user_products {
                // 現在のカートにない商品だけを集計
                if !current_product_ids.contains(&product.product_id) {
                    *product_scores.entry(product.product_id).or_insert(0.0) += user_similarities
                        .iter()
                        .find(|(idx, _)| *idx == user_idx)
                        .map(|(_, score)| *score)
                        .unwrap_or(0.0);
                }
            }
        }
    }

    // スコア順にソートして返す
    let mut suggestions: Vec<(String, f32)> = product_scores.into_iter().collect();

    suggestions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 上位5件に限定
    suggestions.truncate(5);

    suggestions
}

pub fn cosine_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
    if vec1.len() != vec2.len() {
        return 0.0;
    }

    let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(&a, &b)| a * b).sum();

    let magnitude1: f32 = vec1.iter().map(|&x| x * x).sum::<f32>().sqrt();
    let magnitude2: f32 = vec2.iter().map(|&x| x * x).sum::<f32>().sqrt();

    if magnitude1 > 0.0 && magnitude2 > 0.0 {
        dot_product / (magnitude1 * magnitude2)
    } else {
        0.0
    }
}

pub fn combined_similarity(user1: &UserVector, user2: &UserVector, region_weight: f32) -> f32 {
    let product_similarity = cosine_similarity(&user1.product_vector, &user2.product_vector);
    let region_similarity = cosine_similarity(&user1.region_vector, &user2.region_vector);

    // 重み付け合計
    (1.0 - region_weight) * product_similarity + region_weight * region_similarity
}

// ユーザーの購入履歴を取得する関数
async fn fetch_user_purchase_history(
    pool: &Arc<mysql::Pool>,
    product_dimensions: &ProductDimensions,
) -> Result<Vec<UserVector>, mysql::Error> {
    let mut conn = pool.get_conn()?;

    // ユーザーごとの地域情報と購入商品を取得
    let rows = conn.query_map(
        "SELECT 
          u.user_id, 
          u.province_code,
          oi.product_variant_id,
          SUM(oi.quantity) as total_quantity
      FROM 
          users u
      JOIN 
          orders o ON u.user_id = o.user_id
      JOIN 
          order_items oi ON o.order_id = oi.order_id
      WHERE 
          o.status = 'completed'
      GROUP BY 
          u.user_id, u.province_code, oi.product_variant_id
      ORDER BY 
          u.user_id",
        |(user_id, province_code, product_id, quantity): (u64, String, String, u32)| {
            (user_id, province_code, product_id, quantity)
        },
    )?;

    // ユーザーIDごとにグループ化
    let mut user_products: HashMap<u64, (String, Vec<ProductItem>)> = HashMap::new();

    for (user_id, province_code, product_id, quantity) in rows {
        let entry = user_products
            .entry(user_id)
            .or_insert_with(|| (province_code, Vec::new()));

        entry.1.push(ProductItem {
            product_id,
            quantity,
        });
    }

    // 各ユーザーのベクトルを作成
    let user_vectors = user_products
        .into_iter()
        .map(|(_, (province_code, products))| {
            create_user_vector(&province_code, &products, product_dimensions)
        })
        .collect();

    Ok(user_vectors)
}

// 特定ユーザーの商品購入履歴を取得
async fn fetch_user_products(
    pool: &Arc<mysql::Pool>,
    user_id: u64,
) -> Result<Vec<ProductItem>, mysql::Error> {
    let mut conn = pool.get_conn()?;

    let products = conn.exec_map(
        "SELECT 
        oi.product_variant_id,
        SUM(oi.quantity) as total_quantity
    FROM 
        orders o
    JOIN 
        order_products oi ON o.order_id = oi.order_id
    WHERE 
        o.user_id = ?
    GROUP BY 
        oi.product_variant_id",
        (user_id,), // カンマを追加してタプルにする
        |(product_id, quantity): (String, u32)| ProductItem {
            product_id,
            quantity,
        },
    )?;

    Ok(products)
}
