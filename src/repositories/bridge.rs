use sqlx::PgPool;
use eyre::Result;
use crate::models::bridge::BridgeEvent;

// Get bridge events from the database
pub async fn get_bridge_events(
    pool: &PgPool,
    event_type: Option<String>,
    network: Option<String>,
    limit: Option<i64>,
    offset: i64,
) -> Result<Vec<BridgeEvent>> {
    let bridge_events = sqlx::query_as!(
        BridgeEvent,
        r#"
            SELECT id, event_type, network, token_address, from_address, to_address, 
                amount, nonce, block_number, tx_hash, source_token, target_token, target_amount
            FROM bridge_events
            WHERE ($1::text IS NULL OR event_type = $1)
            AND ($2::text IS NULL OR network = $2)
            ORDER BY block_number DESC, nonce DESC
            LIMIT $3
            OFFSET $4
        "#,
        event_type,
        network,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(bridge_events)
}

// Save a bridge event to the database
pub async fn save_bridge_event(pool: &PgPool, event: &BridgeEvent) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO bridge_events 
            (event_type, network, token_address, from_address, to_address, amount, nonce, block_number, tx_hash, 
             source_token, target_token, target_amount) 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
        event.event_type,
        event.network,
        event.token_address,
        event.from_address,
        event.to_address,
        event.amount,
        event.nonce,
        event.block_number,
        event.tx_hash,
        event.source_token,
        event.target_token,
        event.target_amount
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Save a batch of bridge events to the database
pub async fn save_batch(pool: &PgPool, events: &[BridgeEvent]) -> Result<()> {
    // Start a transaction
    let mut tx = pool.begin().await?;

    for event in events {
        sqlx::query!(
            r#"
            INSERT INTO bridge_events 
                (event_type, network, token_address, from_address, to_address, amount, nonce, block_number, tx_hash,
                 source_token, target_token, target_amount) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            event.event_type,
            event.network,
            event.token_address,
            event.from_address,
            event.to_address,
            event.amount,
            event.nonce,
            event.block_number,
            event.tx_hash,
            event.source_token,
            event.target_token,
            event.target_amount
        )
        .execute(&mut *tx)
        .await?;
    }

    // Commit the transaction
    tx.commit().await?;
    Ok(())
} 