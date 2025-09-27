use std::{collections::HashMap, sync::LazyLock};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plan {
    pub id: i32,
    pub name: String,
    pub price: i32, // cent
    pub description: String,
}

pub static PLANS: LazyLock<DashMap<i32, Plan>> = LazyLock::new(|| {
    let map = DashMap::new();

    // 读取并解析 TOML 文件
    let content = std::fs::read_to_string("./data/plans.toml").expect("Failed to read plans.toml");
    let plans: HashMap<String, Vec<Plan>> = toml::from_str(&content).expect("Failed to parse TOML");

    // 将套餐添加到 DashMap 中，使用id作为键
    for plan in plans.get("plans").unwrap() {
        map.insert(plan.id, plan.clone());
    }

    map
});

pub fn get_plans() -> Vec<Plan> {
    PLANS.iter().map(|entry| entry.value().clone()).collect()
}

pub fn get_plan_by_id(id: i32) -> Option<Plan> {
    PLANS.get(&id).map(|entry| entry.value().clone())
}
