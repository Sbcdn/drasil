pub mod adm;
pub mod dapi;
pub mod discounts;
pub mod mint;
pub mod rwd;
pub mod whitelist;

use std::{collections::HashMap, sync::Arc, env::var};
use tokio::sync::{mpsc, Mutex};
use warp::ws::Message;

use crate::error::Error;
use crate::Result;
use deadpool_lapin::Pool;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref AMQP_ADDR: String =
        var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rmq:rmq@127.0.0.1:5672/%2f".into());
    pub static ref QUEUE_NAME: String =
        var("QUEUE_NAME").unwrap_or_else(|_| "mint_response".to_string());
    pub static ref CONSUMER_NAME: String =
        var("CONSUMER_NAME").unwrap_or_else(|_| "work_loki_0".to_string());
}
#[derive(Debug, Clone)]
pub struct Client {
    pub client_id: String,
    pub user_id: u64,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}
pub type Connection = deadpool::managed::Object<deadpool_lapin::Manager>;
pub type Clients = Arc<Mutex<HashMap<String, Client>>>;

pub async fn get_user_from_string(us: &str) -> Result<i64> {
    let user = match us.parse::<i64>() {
        Ok(u) => u,
        Err(_) => return Err(Error::Custom("invalid user".to_string())),
    };

    Ok(user)
}

async fn get_rmq_con(pool: Pool) -> std::result::Result<Connection, deadpool_lapin::PoolError> {
    let connection = pool.get().await?;
    Ok(connection)
}
