/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use crate::MurinError;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::address as caddr;
use clib::utils::BigNum;
use serde::{Deserialize, Serialize};
use std::{fmt, str};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CMintHandle {
    pub id: i64,
    pub project_id: i64,
    pub pay_addr: String,
    pub nft_ids: Vec<String>,
    pub v_nfts_b: Vec<String>,
}

impl fmt::Display for CMintHandle {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let serde = serde_json::to_string(self)
            .expect("Error: could not convert Rewardhandle to JSON string");
        fmt.write_str(&serde)?;
        Ok(())
    }
}

impl From<String> for CMintHandle {
    fn from(str: String) -> Self {
        let rwd: CMintHandle = serde_json::from_str(&str)
            .expect("Error: could not convert JSON string to Rewardhandle");
        rwd
    }
}

impl CMintHandle {
    pub fn payment_addr(&self) -> Result<clib::address::RewardAddress, MurinError> {
        let addr = crate::b_decode_addr_na(&self.pay_addr)?;
        clib::address::RewardAddress::from_address(&addr).ok_or_else(|| {
            MurinError::new("Error: could not construct RewardAddress for CMintHandle")
        })
    }

    pub fn total_value(handles: &[CMintHandle]) -> Result<clib::utils::Value, MurinError> {
        let tv: clib::utils::Value =
            handles
                .iter()
                .fold(clib::utils::Value::zero(), |mut acc, n| {
                    let v = n
                        .v_nfts_b
                        .iter()
                        .fold(clib::utils::Value::zero(), |mut acc, m| {
                            acc = acc
                                .checked_add(
                                    &clib::utils::Value::from_bytes(hex::decode(m).unwrap())
                                        .unwrap(),
                                )
                                .unwrap();
                            acc
                        });

                    acc = acc.checked_add(&v).unwrap();
                    acc
                });
        Ok(tv)
    }

    pub fn value(&self) -> Result<clib::utils::Value, MurinError> {
        let v = self
            .v_nfts_b
            .iter()
            .fold(clib::utils::Value::zero(), |mut acc, m| {
                acc = acc
                    .checked_add(&clib::utils::Value::from_bytes(hex::decode(m).unwrap()).unwrap())
                    .unwrap();
                acc
            });
        Ok(v)
    }

    pub fn nft_ids(&self) -> Result<Vec<clib::AssetName>, MurinError> {
        let mut names = Vec::<clib::AssetName>::new();
        for n in &self.nft_ids {
            names.push(clib::AssetName::new(hex::decode(n)?)?);
        }
        Ok(names)
    }
}

#[derive(Debug, Clone)]
pub struct PriceCMintHandle {
    pub handle_id: i64,
    pub price: BigNum,
    pub seller_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColMinterTxData {
    pub mint_handles: Vec<CMintHandle>,
    pub claim_stake_addr: caddr::Address,
}

impl ColMinterTxData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mint_handles: Vec<CMintHandle>,
        claim_stake_addr: caddr::Address,
    ) -> ColMinterTxData {
        ColMinterTxData {
            mint_handles,
            claim_stake_addr,
        }
    }
}

impl ToString for ColMinterTxData {
    fn to_string(&self) -> String {
        serde_json::to_string(&serde_json::json!(self)).unwrap()
    }
}

impl core::str::FromStr for ColMinterTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> std::result::Result<Self, Self::Err> {
        serde_json::from_str::<ColMinterTxData>(src)
            .map_err(|_| MurinError::new("Couldn't convert String to ColMinterTxData"))
    }
}
