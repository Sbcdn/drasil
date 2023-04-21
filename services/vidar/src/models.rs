use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct QAddresses {
    pub addresses: String,
}
#[derive(Deserialize, Debug, Clone)]
pub struct QStakeAddress {
    pub stake_address: String,
}
