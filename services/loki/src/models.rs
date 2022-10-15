use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use warp::ws::Message;

#[derive(Debug, Clone)]
pub struct Client {
    pub client_id: String,
    pub user_id: u64,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

pub type Clients = Arc<Mutex<HashMap<String, Client>>>;

pub type Connection = deadpool::managed::Object<deadpool_lapin::Manager>;

#[derive(Serialize, Debug)]
pub struct ErrorResult {
    pub detail: String,
}

#[derive(Serialize)]
pub struct JWTToken {
    pub token: String,
    pub creation_date: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub enum WSCom {
    Alive,
    ClaimMintRewards(ClaimMintRewards),
}

#[derive(Serialize, Deserialize)]
pub struct ClaimMintRewards {
    pub mpid: i64,
    pub claim_addr: String,
    pub user_id: Option<i64>,
}
