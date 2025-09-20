use std::sync::OnceLock;

use tokio_rusqlite::{Connection, params};

static CONN: OnceLock<Connection> = OnceLock::new();
use anyhow::Result;
use anyhow::anyhow;

use crate::cdk::CDK;
use crate::order::Order;

pub async fn init_db() -> Result<()> {
    // Open the connection asynchronously first
    let conn = Connection::open("./chinpay.db").await?;
    let _ = conn
        .call(|conn| {
            Ok(conn.execute_batch(
                "
            CREATE TABLE IF NOT EXISTS order (
                uuid TEXT PRIMARY KEY,
                timestamp DATETIME NOT NULL,
                afd_order TEXT NOT NULL,
                price REAL NOT NULL,
                cdk TEXT NOT NULL,
                status VARCHAR(10) NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cdk (
                cdk        TEXT NOT NULL UNIQUE,
                uuid       TEXT,
                used_by    TEXT
            );
            ",
            ))
        })
        .await?;
    // Store the connection in the OnceLock
    CONN.get_or_init(|| conn);
    Ok(())
}

pub async fn insert_order(order: Order) -> Result<()> {
    let conn = get_conn()?;
    conn.call(move |conn| {
        let cdk_str = match &order.cdk {
            Some(cdk) => cdk.to_string(),
            None => "NULL".to_string(),
        };

        conn.execute(
            "INSERT INTO `order` (uuid, timestamp, afd_order, price, cdk, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                order.uuid,
                order.timestamp,
                order.afd_order,
                order.price,
                cdk_str,
                order.status.to_string()
            ],
        )?;
        Ok(())
    })
    .await?;

    Ok(())
}

pub async fn insert_cdk(uuid: String, cdk: CDK) -> Result<()> {
    let conn = get_conn()?;
    conn.call(move |conn| {
        conn.execute(
            "INSERT INTO `cdk` (cdk, uuid, used_by) VALUES (?1, ?2, NULL)",
            params![cdk.to_string(), uuid,],
        )?;
        Ok(())
    })
    .await?;

    Ok(())
}

pub async fn use_cdk(cdk: String, user: String) -> Result<()> {
    let conn = get_conn()?;
    let lines = conn
        .call(move |conn| {
            let lines = conn.execute(
                "UPDATE `cdk` SET `used_by` = ?1 WHERE `cdk` = ?2 AND `used_by` IS NULL",
                params![user, cdk],
            )?;

            Ok(lines)
        })
        .await?;

    if lines == 0 {
        return Err(anyhow!("cdk already used or not exist"));
    }

    Ok(())
}

fn get_conn() -> Result<Connection> {
    let conn = CONN.get();
    if conn.is_none() {
        return Err(anyhow!("connection not initialized"));
    }

    Ok(conn.unwrap().clone())
}
