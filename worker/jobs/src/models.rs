use serde::{Deserialize, Serialize};

pub type Connection = deadpool::managed::Object<deadpool_lapin::Manager>;

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResult {
    pub detail: String,
}
