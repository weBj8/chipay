mod cdk;
mod dao;
mod order;
mod plan;
use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_embed::ServeEmbed;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use tokio::time::interval;
use tracing::{debug, info, level_filters::LevelFilter, warn};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

use dashmap::DashMap;

use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

static ORDER_MAP: LazyLock<DashMap<String, (Order, Instant)>> = LazyLock::new(|| DashMap::new());

use crate::{
    order::Order,
    plan::{Plan, get_plan_by_id},
};

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

async fn check_timeout() {
    // 5 分钟的超时时间
    const ORDER_TIMEOUT_SECS: u64 = 5 * 60;
    // 清理任务每 5s 运行一次
    const CLEANUP_INTERVAL_SECS: u64 = 5;
    let mut interval = interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));

    interval.tick().await;

    loop {
        interval.tick().await;

        debug!("Starting ORDER_MAP cleanup...");
        let now = Instant::now();
        let timeout = Duration::from_secs(ORDER_TIMEOUT_SECS);

        // 使用 DashMap 的 retain 方法来原子性地进行清理
        let mut removed_count = 0;
        ORDER_MAP.retain(|_order_id, order| {
            if now.duration_since(order.1) > timeout {
                info!("Order {} timed out and is being removed.", _order_id);
                removed_count += 1;
                return false;
            }

            return true;
        });

        debug!(
            "Finished cleanup. Removed {} timed out orders.",
            removed_count
        );
    }
}

async fn handle_webhook(request: Json<WebhookRequest>) -> Result<Json<WebhookResponse>, AppError> {
    let afd_id = &request.data.order.out_trade_no;
    let price: i32 = request.data.order.total_amount.replace(".", "").parse()?;
    let uuid = request
        .data
        .order
        .custom_order_id
        .as_deref()
        .unwrap_or_default();

    info!("Webhook triggered");
    debug!("{:?}", request);

    let response = WebhookResponse::new(200);
    debug!("{:?}", response);

    if let Some(mut entry) = ORDER_MAP.get_mut(uuid) {
        let order = &mut entry.0;
        if order.price != price {
            order.status = order::Status::Failed;
            warn!(
                "Order {} with afd id {} FAILED due to diffrent price",
                uuid, afd_id
            );
            dao::insert_order(order.to_owned()).await?;
            return Ok(response.into());
        }
        order.status = order::Status::Completed;
        order.cdk = Some(cdk::CDK::new());
        order.cdk.as_mut().unwrap().plan = get_plan_by_id(order.plan.clone().unwrap().id);
        info!("Order {} with afd id {} completed", uuid, afd_id);
        dao::insert_order(order.to_owned()).await?;
        dao::insert_cdk(order.uuid.to_string(), order.cdk.clone().unwrap()).await?;
    }

    Ok(response.into())
}

async fn create_order(request: Json<order::OrderRequest>) -> Json<Order> {
    let order = Order::new(((request.price * 100.0).round()) as i32, request.plan_id);

    info!("Order {} created with plan {}, price {}", order.uuid, request.plan_id, request.price);

    ORDER_MAP.insert(order.uuid.to_string(), (order.clone(), Instant::now()));

    order.into()
}

async fn get_order_status(Path(order_id): Path<String>) -> String {
    if let Some(entry) = ORDER_MAP.get(&order_id) {
        let order = &entry.0;
        let order_str = order.status.to_string();
        if order.status == order::Status::Completed {
            let order_id_clone = order_id.clone();
            tokio::task::spawn_blocking(move || {
                let _ = tokio::time::sleep(tokio::time::Duration::from_secs(60));
                ORDER_MAP.remove(&order_id_clone);
                info!("Order {} removed", order_id_clone);
            });
        }

        order_str
    } else {
        order::Status::NotFound.to_string()
    }
}

async fn get_order_cdk(Path(order_id): Path<String>) -> String {
    debug!("Querying cdk for uuid {}", order_id);
    let order_id_clone = order_id.clone();
    match dao::query_cdk_from_uuid(order_id).await {
        Ok(Some(cdk)) => {
            debug!("CDK found for {} is {}", order_id_clone, cdk.cdk);
            cdk.to_string()
        }
        Ok(None) => "".to_string(),
        Err(_) => "".to_string(),
    }
}

async fn use_cdk(request: Json<CdkUseRequest>) -> Result<String, AppError> {
    let plan = dao::use_cdk(request.cdk.to_owned(), request.user.to_owned()).await?;
    debug!("CDK {} with plan {} used", request.cdk, plan);
    Ok(plan.to_string())
}

async fn get_plans() -> Json<Vec<Plan>> {
    let plans = plan::get_plans();
    Json(plans)
}

async fn check_cdk() -> Result<String, AppError> {
    todo!("check cdk")
}

async fn get_order_details() -> Result<String, AppError> {
    todo!("get order details")
}

#[derive(RustEmbed, Clone)]
#[folder = "frontend/"]
struct Assets;

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

    let serve_assets = ServeEmbed::<Assets>::new();

    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(async || "sb"))
        .fallback_service(serve_assets)
        .route("/api/webhook", post(handle_webhook))
        .route("/api/create_order", post(create_order))
        .route("/api/get_order_status/{order_uuid}", get(get_order_status))
        .route("/api/get_order_cdk/{order_uuid}", get(get_order_cdk))
        .route("/api/use_cdk", post(use_cdk))
        .route("/api/get_plans", get(get_plans))
        .route("/api/admin/cdk_details", post(check_cdk))
        .route("/api/admin/order_details", post(get_order_details));

    tokio::spawn(async move { check_timeout().await });
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
