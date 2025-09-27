use std::path::Path;
use std::sync::OnceLock;
use tokio_rusqlite::{Connection, params};

static CONN: OnceLock<Connection> = OnceLock::new();
use anyhow::Result;
use anyhow::anyhow;

use crate::cdk::CDK;
use crate::order::Order;

pub async fn init_db() -> Result<()> {
    // Open the connection asynchronously first
    let conn = Connection::open("./data/chinpay.db").await?;
    let _result = conn
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
                used_time  DATETIME,
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
            "INSERT INTO `cdk` (cdk, uuid , plan, used_by, used_time) VALUES (?1, ?2, ?3, NULL, NULL)",
            params![cdk.to_string(), uuid, cdk.plan.unwrap().id],
        )?;
        Ok(())
    })
    .await?;

    Ok(())
}

pub async fn query_cdk_from_uuid(uuid: String) -> Result<Option<CDK>> {
    let conn = get_conn()?;

    let result = conn
        .call(move |conn| {
            let mut stmt = conn.prepare("SELECT cdk, plan FROM `cdk` WHERE `uuid` = ?1")?;
            let mut rows = stmt.query(params![uuid])?;

            if let Some(row) = rows.next()? {
                let cdk_str: String = row.get(0)?;
                let plan_id: i32 = row.get(1)?;

                // Get the plan from the global PLANS map
                let plan = crate::plan::get_plan_by_id(plan_id);

                let cdk = CDK {
                    cdk: cdk_str,
                    used_by: None, // We don't fetch this field as it's not needed for this use case
                    plan,
                };

                Ok(Some(cdk))
            } else {
                Ok(None)
            }
        })
        .await?;

    Ok(result)
}

pub async fn use_cdk(cdk: String, user: String) -> Result<i32> {
    let conn = get_conn()?;

    let cdk_clone = cdk.clone();

    let lines = conn
        .call(move |conn| {
            let lines = conn.execute(
                "UPDATE `cdk` SET `used_by` = ?1, `used_time` = DATETIME('now') WHERE `cdk` = ?2 AND `used_by` IS NULL",
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
