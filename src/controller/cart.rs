use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;

use crate::service;

#[derive(Deserialize)]
pub struct CartRequest {
    pub province_code: String,
    #[serde(deserialize_with = "deserialize_products")]
    pub products: Vec<CartProduct>,
}

#[derive(Deserialize)]
pub struct CartProduct {
    pub product_variant_id: String,
    pub quantity: u32,
}

// カスタムデシリアライザ
fn deserialize_products<'de, D>(deserializer: D) -> Result<Vec<CartProduct>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    serde_json::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Serialize)]
pub struct SuggestionResponse {
    product_variant_id: String,
    score: f32,
}

#[derive(Serialize)]
pub struct ApiResponse {
    message: String,
    suggestions: Vec<SuggestionResponse>,
}

pub async fn get_suggestions(
    State(pool): State<Arc<mysql::Pool>>,
    Query(params): Query<CartRequest>,
) -> Json<ApiResponse> {
    // 商品次元情報を取得
    let product_dimensions = match service::cart::fetch_product_dimensions(&pool).await {
        Ok(dimensions) => dimensions,
        Err(err) => {
            return Json(ApiResponse {
                message: format!("Error fetching product dimensions: {}", err),
                suggestions: vec![],
            });
        }
    };

    // CartProductをProductItemに変換
    let product_items: Vec<service::cart::ProductItem> = params
        .products
        .iter()
        .map(|p| service::cart::ProductItem {
            product_variant_id: p.product_variant_id.clone(),
            quantity: p.quantity,
        })
        .collect();

    // 現在のユーザーベクトルを作成
    let current_user = service::cart::create_order_vector(
        &params.province_code,
        &product_items,
        &product_dimensions,
    );

    // 他のユーザーの履歴を取得して類似度を計算
    let similar_product_scores = service::cart::get_similar_products(
        &pool,
        &current_user,
        &product_items,
        &product_dimensions,
    )
    .await;

    println!("{}件の類似商品を取得しました", similar_product_scores.len());

    let suggestions = similar_product_scores
        .into_iter()
        .map(|(product_id, score)| SuggestionResponse {
            product_variant_id: product_id,
            score,
        })
        .collect();

    Json(ApiResponse {
        message: "Successfully generated suggestions".to_string(),
        suggestions: suggestions,
    })
}
