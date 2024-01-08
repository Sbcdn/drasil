use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct OptimizeRewardUTxOs {
    pub ids: Vec<i64>,
}
