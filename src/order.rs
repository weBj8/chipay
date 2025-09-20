use std::cell::OnceCell;
use uuid::Uuid;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub price: f32,
    pub afd_order: String,
    pub cdk: String,
    pub status: Status,
}

#[derive(Deserialize, Debug)]
pub struct OrderRequest {
    pub price: f32,
}

impl Order {
    pub fn new(price: f32) -> Self {
        let uuid = Uuid::new_v4().to_string();
        let utc_now: DateTime<Utc> = Utc::now();
        let timestamp_secs = utc_now.timestamp();

        Self {
            uuid: uuid,
            timestamp: timestamp_secs,
            price: price,
            afd_order: "".into(),
            cdk: "".into(),
            status: Status::Pending,
        }
    }
}
