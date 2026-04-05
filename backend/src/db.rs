use sqlx::PgPool;

pub async fn migrate(pool: &PgPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS escrows (
            address TEXT PRIMARY KEY,
            creator TEXT NOT NULL,
            recipient TEXT NOT NULL,
            mint TEXT NOT NULL,
            amount BIGINT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            escrow_id BIGINT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            expires_at TIMESTAMPTZ,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_escrows_creator ON escrows(creator);
        CREATE INDEX IF NOT EXISTS idx_escrows_recipient ON escrows(recipient);
        CREATE INDEX IF NOT EXISTS idx_escrows_status ON escrows(status);
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to run migrations");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id BIGSERIAL PRIMARY KEY,
            escrow_address TEXT NOT NULL,
            event_type TEXT NOT NULL,
            tx_signature TEXT NOT NULL,
            data JSONB,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_events_escrow ON events(escrow_address);
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create events table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS webhooks (
            id BIGSERIAL PRIMARY KEY,
            url TEXT NOT NULL,
            filter_creator TEXT,
            filter_recipient TEXT,
            secret TEXT NOT NULL,
            active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create webhooks table");

    tracing::info!("Database migrations complete");
}
