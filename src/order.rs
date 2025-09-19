use std::cell::OnceCell;
use uuid::Uuid;

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub enum Status {
    Pending,
    Completed,
    Failed,
}

#[derive(Serialize, Debug, Clone)]
pub struct Order {
    pub uuid: String,
    pub timestamp: i64,
    pub afd_order: String,
    pub cdk: String,
    pub status: Status,
}

impl Order {
    pub fn new() -> Self {
        let uuid = Uuid::new_v4().to_string();
        let utc_now: DateTime<Utc> = Utc::now();
        let timestamp_secs = utc_now.timestamp();

        Self {
            uuid: uuid,
            timestamp: timestamp_secs,
            afd_order: "".into(),
            cdk: "".into(),
            status: Status::Pending,
        }
    }
}
