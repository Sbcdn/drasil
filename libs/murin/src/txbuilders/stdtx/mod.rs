use std::{fmt::Display, str::FromStr};

use cardano_serialization_lib::{address::Address, utils::BigNum, AssetName, PolicyID};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub mod build_cpo;
pub mod build_wallet_asset_transfer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdAssetHandle {
    pub fingerprint: Option<String>,
    pub policy: Option<PolicyID>,
    pub tokenname: Option<AssetName>,
    pub amount: BigNum,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTransfer {
    pub receiver: Address,
    pub assets: Vec<StdAssetHandle>,
    pub metadata: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTxData {
    pub wallet_addresses: Vec<Address>,
    pub transfers: Vec<AssetTransfer>,
}

impl Display for StandardTxData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", json!(&self))
    }
}

impl FromStr for StandardTxData {
    type Err = crate::MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str::<StandardTxData>(src)?)
    }
}
