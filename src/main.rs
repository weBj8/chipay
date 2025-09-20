mod cdk;
mod dao;
mod order;
use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{self, get, post},
};
use serde::{Deserialize, Serialize};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

use dashmap::DashMap;

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

#[derive(Deserialize, Debug)]
struct CdkUseRequest {
    pub cdk: String,
    pub user: String,
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

const DEFAULT_LOG_LEVEL: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::DEBUG
} else {
    LevelFilter::INFO
};

async fn handle_webhook(request: Json<WebhookRequest>) -> Result<Json<WebhookResponse>, AppError> {
    let afd_id = &request.data.order.out_trade_no;
    let price: i32 = request.data.order.total_amount.replace(".", "").parse()?;
    let uuid = request
        .data
        .order
        .custom_order_id
        .as_deref()
        .unwrap_or_default();

    info!("webhook recived");
    info!("{:?}", request);

    let response = WebhookResponse::new(200);
    info!("{:?}", response);

    if let Some(mut order) = ORDER_MAP.get_mut(uuid) {
        if order.price != price {
            order.status = order::Status::Failed;
            warn!("order {} with afd id {} failed due to diffrent price", uuid, afd_id);
            dao::insert_order(order.to_owned()).await?;
            return Ok(response.into());
        }
        order.status = order::Status::Completed;
        order.cdk = Some(cdk::CDK::new());
        info!("order {} with afd id {} completed", uuid, afd_id);
        dao::insert_order(order.to_owned()).await?;
        dao::insert_cdk(order.uuid.to_string(), order.cdk.clone().unwrap()).await?;
    }

    Ok(response.into())
}

async fn create_order(request: Json<order::OrderRequest>) -> Json<Order> {
    let order = Order::new(((request.price * 100.0).round()) as i32);

    ORDER_MAP.insert(order.uuid.to_string(), order.clone());

    order.into()
}

async fn get_order_status(Path(order_id): Path<String>) -> String {
    if let Some(order) = ORDER_MAP.get(&order_id) {
        let order_str = order.status.to_string();
        if order.status == order::Status::Completed {
            ORDER_MAP.remove(&order_id);
        }

        order_str
    } else {
        order::Status::NotFound.to_string()
    }
}

async fn get_order_cdk(Path(order_id): Path<String>) -> String {
    let order = ORDER_MAP.get(&order_id);

    if order.is_none() {
        return "".to_string();
    }

    let order = order.unwrap();

    if order.status != order::Status::Completed || order.cdk.is_none() {
        return "".to_string();
    }

    order.cdk.as_ref().unwrap().to_string()
}

async fn use_cdk(request: Json<CdkUseRequest>) -> Result<StatusCode, AppError> {
    dao::use_cdk(request.cdk.to_owned(), request.user.to_owned()).await?;
    Ok(StatusCode::OK)
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
        .route("/api/get_order_status/{order_uuid}", get(get_order_status))
        .route("/api/get_order_cdk/{order_uuid}", get(get_order_cdk))
        .route("/api/use_cdk", post(use_cdk));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
