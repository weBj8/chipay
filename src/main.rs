mod dao;
mod order;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{any, get, post},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::Subscriber;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

use dashmap::DashMap;
use uuid::Uuid;

use std::sync::LazyLock;

static ORDER_MAP: LazyLock<DashMap<String, Order>> = LazyLock::new(|| DashMap::new());

use crate::order::Order;

#[derive(Deserialize, Debug)]
pub struct WebhookRequest {
    pub ec: i32,
    pub em: String,
    pub data: WebhookData,
}

#[derive(Deserialize, Debug)]
pub struct WebhookData {
    #[serde(rename = "type")]
    pub data_type: String,
    pub order: AfdOrder,
}

#[derive(Deserialize, Debug)]
pub struct AfdOrder {
    pub out_trade_no: String,
    pub custom_order_id: Option<String>,
    pub user_id: String,
    pub user_private_id: String,
    pub plan_id: String,
    pub month: i32,
    pub total_amount: String,
    pub show_amount: String,
    pub status: i32,
    pub remark: String,
    pub redeem_id: String,
    pub product_type: i32,
    pub discount: String,
    pub sku_detail: Vec<SkuDetail>,
    pub address_person: String,
    pub address_phone: String,
    pub address_address: String,
}

#[derive(Deserialize, Debug)]
pub struct SkuDetail {
    pub sku_id: String,
    pub count: i32,
    pub name: String,
    pub album_id: String,
    pub pic: String,
}

#[derive(Serialize, Debug)]
pub struct WebhookResponse {
    pub ec: i32,
}

impl WebhookResponse {
    pub fn new(ec: i32) -> Self {
        Self { ec }
    }
}

const DEFAULT_LOG_LEVEL: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::DEBUG
} else {
    LevelFilter::INFO
};

async fn handle_webhook(request: Json<WebhookRequest>) -> Json<WebhookResponse> {
    let afd_id = &request.data.order.out_trade_no;
    let uuid = request
        .data
        .order
        .custom_order_id
        .as_deref()
        .unwrap_or_default();

    info!("webhook recived");
    info!("{:?}", request);

    if let Some(mut order) = ORDER_MAP.get_mut(uuid) {
        order.status = order::Status::Completed;
    }

    let response = WebhookResponse::new(200);
    info!("{:?}", response);

    response.into()
}

async fn create_order(request: Json<order::OrderRequest>) -> Json<Order> {
    let order = Order::new(request.price);

    ORDER_MAP.insert(order.uuid.to_string(), order.clone()); 

    order.into()
}

async fn get_order_status(Path(order_id): Path<String>) -> String {
    if let Some(order) = ORDER_MAP.get(&order_id) {
        if order.status == order::Status::Pending {
            "pending".to_string()
        } else if order.status == order::Status::Completed {
            ORDER_MAP.remove(&order_id);
            
            "completed".to_string()
        } else {
            "failed".to_string()
        }
    } else {
        "not found".to_string()
    }
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::registry()
        .with(
            EnvFilter::builder()
                .with_default_directive(DEFAULT_LOG_LEVEL.into())
                .from_env_lossy(),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    dao::init_db().await.expect("Failed to initialize database");

    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(async || "sb"))
        .route("/api/webhook", post(handle_webhook))
        .route("/api/create_order", post(create_order))
        .route("/api/get_order_status/{order_uuid}", get(get_order_status));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
