use std::sync::OnceLock;

use tokio_rusqlite::{params, Connection};

static CONN: OnceLock<Connection> = OnceLock::new();
use anyhow::Result;

pub async fn init_db() -> Result<()> {
    // Open the connection asynchronously first
    let conn = Connection::open("./chinpay.db").await?;
    let _ = conn.call(|conn| {
        Ok(conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS order (
                uuid TEXT PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                afd_order TEXT NOT NULL,
                cdk TEXT NOT NULL,
                status INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cdk (
                cdk        TEXT NOT NULL UNIQUE,
                used_by     TEXT
            );
            "
        ))
    }).await?;
    // Store the connection in the OnceLock
    CONN.get_or_init(|| conn);
    Ok(())
}