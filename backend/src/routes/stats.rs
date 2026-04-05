use actix_web::{get, web, HttpResponse};
use serde::Serialize;
use crate::AppState;

#[derive(Serialize)]
pub struct ProtocolStats {
    pub total_escrows: i64,
    pub active_escrows: i64,
    pub total_volume: i64,
}

#[get("/stats")]
pub async fn get_stats(state: web::Data<AppState>) -> HttpResponse {
    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM escrows")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let active = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM escrows WHERE status = 'active'")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let volume = sqlx::query_scalar::<_, Option<i64>>("SELECT SUM(amount) FROM escrows")
        .fetch_one(&state.db)
        .await
        .unwrap_or(None)
        .unwrap_or(0);

    HttpResponse::Ok().json(ProtocolStats {
        total_escrows: total,
        active_escrows: active,
        total_volume: volume,
    })
}
