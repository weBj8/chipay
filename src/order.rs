use std::fmt;
use uuid::Uuid;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    cdk::CDK,
    plan::{Plan, get_plan_by_id},
};

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub enum Status {
    Pending,
    Completed,
    Failed,
    NotFound,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Status::Pending => "pending",
                Status::Completed => "completed",
                Status::Failed => "failed",
                Status::NotFound => "not found",
            }
        )
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Order {
    pub uuid: String,
    pub timestamp: i64,
    pub price: i32, // cent
    pub afd_order: String,
    pub cdk: Option<CDK>,
    pub plan: Option<Plan>,
    pub status: Status,
}

#[derive(Deserialize, Debug)]
pub struct OrderRequest {
    pub price: f32,
    pub plan_id: i32,
}

impl Order {
    pub fn new(price: i32, plan_id: i32) -> Self {
        let uuid = Uuid::new_v4().to_string();
        let utc_now: DateTime<Utc> = Utc::now();
        let timestamp_secs = utc_now.timestamp();

        Self {
            uuid: uuid,
            timestamp: timestamp_secs,
            price: price,
            afd_order: "".into(),
            cdk: None,
            plan: get_plan_by_id(plan_id),
            status: Status::Pending,
        }
    }
}
