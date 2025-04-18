use crate::db;
use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

// JSONレスポンス用の構造体
#[derive(Serialize)]
pub struct UserResponse {
    id: i32,
    name: String,
    email: String,
    api_token: String,
}

// レスポンス全体の構造体
#[derive(Serialize)]
pub struct ApiResponse {
    message: String,
    users: Vec<UserResponse>,
}

// ルートパスのハンドラ - JSONを返すように変更
pub async fn get_users(pool: State<Arc<mysql::Pool>>) -> Json<ApiResponse> {
    // ユーザー一覧を取得
    let users_result = db::get_users(pool.0.clone()).await;

    match users_result {
        Ok(users) => {
            // ユーザーデータをUserResponse構造体に変換
            let user_responses: Vec<UserResponse> = users
                .into_iter()
                .map(|user| UserResponse {
                    id: user.id,
                    name: user.name,
                    email: user.email,
                    api_token: user.api_token.unwrap_or("".to_string()),
                })
                .collect();

            // JSONレスポンスを返す
            Json(ApiResponse {
                message: "Successfully retrieved users".to_string(),
                users: user_responses,
            })
        }
        Err(e) => {
            // エラーの場合も適切なJSONレスポンスを返す
            Json(ApiResponse {
                message: format!("データベースエラー: {}", e),
                users: Vec::new(),
            })
        }
    }
}
