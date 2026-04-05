use sqlx::PgPool;

/// Placeholder indexer — subscribes to program logs via Solana WebSocket
/// and indexes escrow events into PostgreSQL.
///
/// In production, this would use `logsSubscribe` to stream real-time events,
/// parse Anchor event data, and upsert into the database.
pub async fn run_indexer(
    _pool: PgPool,
    rpc_url: &str,
    program_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(
        "Indexer started for program {} on {}",
        program_id,
        rpc_url
    );

    // TODO: Implement WebSocket subscription to program logs
    // 1. Connect to Solana WebSocket (replace https:// with wss://)
    // 2. Subscribe to logsSubscribe with program_id filter
    // 3. Parse Anchor events from log messages
    // 4. Upsert escrow state into PostgreSQL
    // 5. Dispatch webhooks on state changes

    // For now, just keep the task alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        tracing::debug!("Indexer heartbeat");
    }
}
