use bigdecimal::BigDecimal;
use cardano_serialization_lib::{crypto::ScriptHash, utils::BigNum, AssetName};
use diesel::Queryable;

pub type Token = (ScriptHash, AssetName, BigNum);
pub type Tokens = Vec<Token>;

#[derive(Queryable, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TokenInfoView {
    pub fingerprint: String,
    pub policy: String,
    pub tokenname: String,
    pub quantity: Option<u64>,
    pub meta_key: Option<i64>,
    pub json: Option<serde_json::Value>,
    pub txhash: Option<String>,
}

#[derive(Queryable, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct EpochStakeView {
    pub stake_addr: String,
    pub amount: BigDecimal,
}

#[derive(Queryable, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DelegationView {
    pub stake_addr: String,
    pub amount: i64,
    pub cert_index: i32,
    pub active_epoch_no: i64,
}

#[derive(Queryable, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct HoldingWalletView {
    pub stake_address: String,
    pub hodl_amount: u64,
    pub policy: String,
    pub tokenname: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Queryable, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CardanoNativeAsset {
    pub id: i64,
    pub policy: Vec<u8>,
    pub name: Vec<u8>,
    pub fingerprint: String,
    pub quantity: BigDecimal,
}
