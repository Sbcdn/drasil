//use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type Connection = deadpool::managed::Object<deadpool_lapin::Manager>;

#[derive(Serialize, Debug)]
pub struct ErrorResult {
    pub detail: String,
}

#[derive(Serialize, Deserialize)]
pub enum WSCom {
    Alive,
    ClaimMintRewards(ClaimMintRewards),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClaimMintRewards {
    pub mpid: i64,
    pub claim_addr: String,
    pub user_id: Option<i64>,
}
