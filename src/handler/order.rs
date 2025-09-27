use serde::{Deserialize, Serialize};

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
pub struct CdkUseRequest {
    pub cdk: String,
    pub user: String,
}

#[derive(Deserialize, Debug)]
pub struct OrderRequest {
    pub price: f32,
    pub plan_id: i32,
}