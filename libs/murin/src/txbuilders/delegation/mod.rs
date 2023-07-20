pub mod build_deleg;
pub use build_deleg::{AtDelegBuilder, AtDelegParams};

use cardano_serialization_lib::crypto as ccrypto;

use crate::MurinError;

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
