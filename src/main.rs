use axum::{routing::get, Router};
use dotenv::dotenv;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod config;
mod db;
mod handlers;

#[tokio::main]
async fn main() {
    // 環境変数の読み込み
    dotenv().ok();
    let database_url = config::get_database_url();

    // データベースURLをOptsオブジェクトに変換
    let opts = mysql::Opts::from_url(&database_url).expect("不正なデータベースURL");
    // Optsオブジェクトを使ってプールを作成
    let pool = mysql::Pool::new(opts).expect("データベース接続に失敗しました");
    let arc_pool = std::sync::Arc::new(pool);

    let app = Router::new()
        .route("/users", get(handlers::get_users))
        .with_state(arc_pool);

    // TcpListenerを使用（axum 0.7以降）
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("🚀 Server started at http://{} 🚀", addr);
    axum::serve(listener, app).await.unwrap();
}
