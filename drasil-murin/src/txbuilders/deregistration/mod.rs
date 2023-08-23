pub mod build_dereg;
pub use build_dereg::{AtDeregBuilder, AtDeregParams};

use cardano_serialization_lib::crypto as ccrypto;
  
use crate::MurinError;
  
#[derive(Debug, Clone)]
pub struct DeregTxData {
    poolhash: String,
    poolkeyhash: ccrypto::Ed25519KeyHash,
    registered: Option<bool>,
}
  
impl DeregTxData {
    pub fn new(poolhash: &str) -> Result<DeregTxData, MurinError> {
        let pool_keyhash = ccrypto::Ed25519KeyHash::from_bech32(poolhash)?;
        Ok(DeregTxData {
            poolhash: poolhash.to_string(),
            poolkeyhash: pool_keyhash,
            registered: None,
        })
    }
   
    pub fn get_poolhash(&self) -> String {
        self.poolhash.clone()
    }
  
    pub fn get_poolkeyhash(&self) -> ccrypto::Ed25519KeyHash {
        self.poolkeyhash.clone()
    }
    
    pub fn get_registered(&self) -> bool {
        if let Some(r) = self.registered {
            r
        } else {
            false
        }
    }

    pub fn set_registered(&mut self, r: Option<bool>) {
        self.registered = r;
    }
}

impl ToString for DeregTxData {
    fn to_string(&self) -> String {
        self.poolhash.clone()
    }
}

impl core::str::FromStr for DeregTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        DeregTxData::new(src)
    }
}
