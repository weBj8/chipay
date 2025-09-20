use std::time::{SystemTime, UNIX_EPOCH};
use md5;
use rand;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDK {
    pub cdk: String,
    pub used_by: Option<String>,
}

impl CDK {
    pub fn new() -> Self {
        let p1 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis().to_string();
        
        let p2 = rand::random::<u64>().to_string();
        let cdk = format!("{:x}", md5::compute(format!("{}{}", p1, p2)));

        Self { cdk, used_by: None }
    }
}

impl std::fmt::Display for CDK {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.cdk)
    }
}