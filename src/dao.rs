use std::sync::OnceLock;
use tokio_rusqlite::types::Null;
use tokio_rusqlite::{Connection, params};
use tracing::{error, info};

static CONN: OnceLock<Connection> = OnceLock::new();
use anyhow::Result;
use anyhow::anyhow;

use crate::cdk::CDK;
use crate::order::Order;
use crate::plan;

pub async fn init_db() -> Result<()> {
    // Open the connection asynchronously first
    let conn = Connection::open("./chinpay.db").await?;
    let result = conn
        .call(|conn| {
            Ok(conn.execute_batch(
                "
            CREATE TABLE IF NOT EXISTS `order` (
                uuid TEXT PRIMARY KEY,
                timestamp DATETIME NOT NULL,
                afd_order TEXT NOT NULL,
                price REAL NOT NULL,
                cdk TEXT NOT NULL,
                plan TEXT,
                status VARCHAR(10) NOT NULL
            );

            CREATE TABLE IF NOT EXISTS `cdk` (
                cdk        TEXT NOT NULL UNIQUE,
                uuid       TEXT,
                plan       INT,
                used_by    TEXT
            );
            ",
            ))
        })
        .await?;

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
            "INSERT INTO `order` (uuid, timestamp, afd_order, price, plan, cdk, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                order.uuid,
                order.timestamp,
                order.afd_order,
                order.price,
                order.plan.as_ref().unwrap().id,
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

    if cdk.plan.is_none() {
        return Err(anyhow!("plan is required"));
    }
    conn.call(move |conn| {
        conn.execute(
            "INSERT INTO `cdk` (cdk, uuid , plan, used_by) VALUES (?1, ?2, ?3, NULL)",
            params![cdk.to_string(), uuid, cdk.plan.unwrap().id],
        )?;
        Ok(())
    })
    .await?;

    Ok(())
}

pub async fn use_cdk(cdk: String, user: String) -> Result<i32> {
    let conn = get_conn()?;

    let cdk_clone = cdk.clone();

    let lines = conn
        .call(move |conn| {
            let lines = conn.execute(
                "UPDATE `cdk` SET `used_by` = ?1 WHERE `cdk` = ?2 AND `used_by` IS NULL",
                params![user, cdk_clone],
            )?;

            Ok(lines)
        })
        .await?;

    if lines == 0 {
        return Err(anyhow!("cdk already used or not exist"));
    }

    let plan = conn
        .call(move |conn| {
            let mut stmt = conn.prepare("SELECT plan FROM `cdk` WHERE `cdk` = ?1")?;
            let mut rows = stmt.query(params![cdk])?;
            for row in rows.next()? {
                let plan: i32 = row.get(0)?;
                return Ok(plan);
            }
            Ok(-1)
        })
        .await?;

    Ok(plan)
}

fn get_conn() -> Result<Connection> {
    let conn = CONN.get();
    if conn.is_none() {
        return Err(anyhow!("connection not initialized"));
    }

    Ok(conn.unwrap().clone())
}
