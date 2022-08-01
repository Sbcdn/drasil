/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
pub mod build_deleg;
pub use build_deleg::build_delegation_tx;

use cardano_serialization_lib::crypto as ccrypto;

use crate::MurinError;

#[derive(Debug, Clone)]
pub struct DelegTxData {
    poolhash: String,
    poolkeyhash: ccrypto::Ed25519KeyHash,
}

impl DelegTxData {
    pub fn new(poolhash: &String) -> Result<DelegTxData, MurinError> {
        let pool_keyhash = ccrypto::Ed25519KeyHash::from_bech32(poolhash)?;
        Ok(DelegTxData {
            poolhash: poolhash.clone(),
            poolkeyhash: pool_keyhash,
        })
    }

    pub fn get_poolhash(&self) -> String {
        self.poolhash.clone()
    }

    pub fn get_poolkeyhash(&self) -> ccrypto::Ed25519KeyHash {
        self.poolkeyhash.clone()
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
        Ok(DelegTxData::new(&src.to_string())?)
    }
}
