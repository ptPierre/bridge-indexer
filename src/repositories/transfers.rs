use sqlx::PgPool;
use eyre::Result;
use crate::models::transfers::Transfer;

// Get transfers from the database
pub async fn get_transfers(pool: &PgPool, limit: Option<i64>, offset: i64) -> Result<Vec<Transfer>> {
    let transfers = sqlx::query_as!(
        Transfer,
        r#"
            SELECT id, from_address, to_address, 
                   value::TEXT as "value!", block_number, tx_hash
            FROM transfers
            ORDER BY block_number DESC
            LIMIT $1
            OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(transfers)
}

// Save a transfer to the database (--> currently I use the batch save only)
pub async fn save_transfer(pool: &PgPool, transfer: &Transfer) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO transfers (from_address, to_address, value, block_number, tx_hash) 
        VALUES ($1, $2, $3, $4, $5)
        "#,
        transfer.from_address,
        transfer.to_address,
        transfer.value,
        transfer.block_number,
        transfer.tx_hash
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Save a batch of transfers to the database
pub async fn save_batch(pool: &PgPool, transfers: &[Transfer]) -> Result<()> {
    // Start a transaction
    let mut tx = pool.begin().await?;

    for transfer in transfers {
        sqlx::query!(
            r#"
            INSERT INTO transfers (from_address, to_address, value, block_number, tx_hash) 
            VALUES ($1, $2, $3, $4, $5)
            "#,
            transfer.from_address,
            transfer.to_address,
            transfer.value,
            transfer.block_number,
            transfer.tx_hash
        )
        .execute(&mut *tx)
        .await?;
    }

    // Commit the transaction
    tx.commit().await?;
    Ok(())
} 