use mysql::prelude::*;
use mysql::*;
use std::sync::Arc;

// ユーザー情報を格納する構造体
#[derive(Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub api_token: Option<String>,
}

// ユーザー一覧を取得する関数
pub async fn get_users(pool: Arc<mysql::Pool>) -> Result<Vec<User>> {
    // MySQLはasyncに対応していないため、tokioのブロッキング実行を使用
    let users = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get_conn()?;

        // usersテーブルからデータを取得
        let users: Vec<User> = conn.query_map(
            "SELECT id, name, email, api_token FROM users",
            |(id, name, email, api_token)| User {
                id,
                name,
                email,
                api_token,
            },
        )?;
        Ok::<Vec<User>, mysql::Error>(users)
    })
    .await
    .expect("ブロッキングタスクの実行に失敗")?;

    Ok(users)
}
