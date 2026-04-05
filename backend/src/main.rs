use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::env;

mod db;
mod routes;
mod indexer;

pub struct AppState {
    pub db: PgPool,
    pub rpc_url: String,
    pub program_id: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let rpc_url = env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let program_id = env::var("PROGRAM_ID")
        .unwrap_or_else(|_| "8Qu8qouNV7CZ4MUEX7rDpAruLFXhvaUruBQRoSViewDY".to_string());
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a number");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    db::migrate(&pool).await;

    let state = web::Data::new(AppState {
        db: pool.clone(),
        rpc_url: rpc_url.clone(),
        program_id: program_id.clone(),
    });

    // Spawn indexer in background
    let indexer_pool = pool.clone();
    let indexer_rpc = rpc_url.clone();
    let indexer_program = program_id.clone();
    tokio::spawn(async move {
        if let Err(e) = indexer::run_indexer(indexer_pool, &indexer_rpc, &indexer_program).await {
            tracing::error!("Indexer error: {}", e);
        }
    });

    tracing::info!("Starting server on port {}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .configure(routes::configure)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
