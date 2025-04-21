use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::get,
};
use dotenv::dotenv;
use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

mod command;
mod config;
mod controller;
mod db;
mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 環境変数の読み込み
    dotenv().ok();

    // コマンドライン引数を取得
    let args: Vec<String> = env::args().collect();

    // 引数が "generate-customers" の場合、その関数を実行
    if args.len() >= 2 && args[1] == "seed" {
        let count = if args.len() >= 3 {
            args[2].parse::<usize>().unwrap_or(100)
        } else {
            100 // デフォルト値
        };

        println!("ユーザーデータ生成を開始します...");
        command::seed::generate_customers(count).await?;
        command::seed::generate_orders(count).await?;
        return Ok(());
    }

    // 通常のサーバー起動処理
    let database_url = config::database::get_database_url();
    let opts = mysql::Opts::from_url(&database_url).expect("不正なデータベースURL");
    let pool = mysql::Pool::new(opts).expect("データベース接続に失敗しました");
    let arc_pool = std::sync::Arc::new(pool);

    // CORSを許可するミドルウェアを設定
    let cors = CorsLayer::new()
        // すべてのオリジンを許可
        .allow_origin(Any)
        // すべてのヘッダーを許可
        .allow_headers(Any)
        // すべてのメソッドを許可
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ]);

    let app = Router::new()
        .route("/users", get(controller::users::get_users))
        .route("/suggestions", get(controller::cart::get_suggestions))
        .with_state(arc_pool)
        .layer(cors); // CORSミドルウェアを追加

    let addr = SocketAddr::from(([127, 0, 0, 1], 3939));
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("🚀 Server started at http://{} 🚀", addr);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
