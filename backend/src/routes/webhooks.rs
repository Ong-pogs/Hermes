use actix_web::{post, delete, web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub filter_creator: Option<String>,
    pub filter_recipient: Option<String>,
}

#[derive(Serialize)]
pub struct WebhookResponse {
    pub id: i64,
    pub url: String,
    pub secret: String,
}

#[post("/webhooks")]
pub async fn create_webhook(
    state: web::Data<AppState>,
    body: web::Json<CreateWebhookRequest>,
) -> HttpResponse {
    let secret = generate_secret();

    match sqlx::query_as::<_, (i64,)>(
        "INSERT INTO webhooks (url, filter_creator, filter_recipient, secret) VALUES ($1, $2, $3, $4) RETURNING id"
    )
    .bind(&body.url)
    .bind(&body.filter_creator)
    .bind(&body.filter_recipient)
    .bind(&secret)
    .fetch_one(&state.db)
    .await
    {
        Ok((id,)) => HttpResponse::Created().json(WebhookResponse {
            id,
            url: body.url.clone(),
            secret,
        }),
        Err(e) => {
            tracing::error!("Failed to create webhook: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create webhook"
            }))
        }
    }
}

#[delete("/webhooks/{id}")]
pub async fn delete_webhook(
    state: web::Data<AppState>,
    id: web::Path<i64>,
) -> HttpResponse {
    match sqlx::query("DELETE FROM webhooks WHERE id = $1")
        .bind(*id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => {
            HttpResponse::Ok().json(serde_json::json!({ "deleted": true }))
        }
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Webhook not found"
        })),
        Err(e) => {
            tracing::error!("Failed to delete webhook: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to delete webhook"
            }))
        }
    }
}

fn generate_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("whsec_{:x}", timestamp)
}
