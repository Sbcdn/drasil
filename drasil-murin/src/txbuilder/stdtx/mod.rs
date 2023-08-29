use std::{fmt::Display, str::FromStr};
pub mod build_deleg;
pub use build_deleg::{AtDelegBuilder, AtDelegParams};
use cardano_serialization_lib::crypto as ccrypto;
use cardano_serialization_lib::{address::Address, utils::BigNum, AssetName, PolicyID};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::MurinError;
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
    pub metadata: Option<String>,
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

#[derive(Debug, Clone)]
pub struct DelegTxData {
    poolhash: String,
    poolkeyhash: ccrypto::Ed25519KeyHash,
    registred: Option<bool>,
}

impl DelegTxData {
    pub fn new(poolhash: &str) -> Result<DelegTxData, MurinError> {
        let pool_keyhash = ccrypto::Ed25519KeyHash::from_bech32(poolhash)?;
        Ok(DelegTxData {
            poolhash: poolhash.to_string(),
            poolkeyhash: pool_keyhash,
            registred: None,
        })
    }

    pub fn get_poolhash(&self) -> String {
        self.poolhash.clone()
    }

    pub fn get_poolkeyhash(&self) -> ccrypto::Ed25519KeyHash {
        self.poolkeyhash.clone()
    }

    pub fn get_registered(&self) -> bool {
        if let Some(r) = self.registred {
            r
        } else {
            false
        }
    }

    pub fn set_registered(&mut self, r: Option<bool>) {
        self.registred = r;
    }
}

impl ToString for DelegTxData {
    fn to_string(&self) -> String {
        self.poolhash.clone()
    }
}

impl core::str::FromStr for DelegTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        DelegTxData::new(src)
    }
}
