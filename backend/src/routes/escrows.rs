use actix_web::{get, web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::AppState;

#[derive(Deserialize)]
pub struct EscrowQuery {
    pub creator: Option<String>,
    pub recipient: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct EscrowRow {
    pub address: String,
    pub creator: String,
    pub recipient: String,
    pub mint: String,
    pub amount: i64,
    pub status: String,
    pub escrow_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[get("/escrows")]
pub async fn get_escrows(
    state: web::Data<AppState>,
    query: web::Query<EscrowQuery>,
) -> HttpResponse {
    let mut sql = String::from("SELECT * FROM escrows WHERE 1=1");
    let mut params: Vec<String> = Vec::new();

    if let Some(ref creator) = query.creator {
        params.push(creator.clone());
        sql.push_str(&format!(" AND creator = ${}", params.len()));
    }
    if let Some(ref recipient) = query.recipient {
        params.push(recipient.clone());
        sql.push_str(&format!(" AND recipient = ${}", params.len()));
    }
    if let Some(ref status) = query.status {
        params.push(status.clone());
        sql.push_str(&format!(" AND status = ${}", params.len()));
    }

    sql.push_str(" ORDER BY escrow_id DESC LIMIT 100");

    let mut query_builder = sqlx::query_as::<_, EscrowRow>(&sql);
    for param in &params {
        query_builder = query_builder.bind(param);
    }

    match query_builder.fetch_all(&state.db).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            tracing::error!("Failed to fetch escrows: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch escrows"
            }))
        }
    }
}

#[get("/escrows/{address}")]
pub async fn get_escrow_by_address(
    state: web::Data<AppState>,
    address: web::Path<String>,
) -> HttpResponse {
    match sqlx::query_as::<_, EscrowRow>("SELECT * FROM escrows WHERE address = $1")
        .bind(address.as_str())
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(row)) => HttpResponse::Ok().json(row),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Escrow not found"
        })),
        Err(e) => {
            tracing::error!("Failed to fetch escrow: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch escrow"
            }))
        }
    }
}
