/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
pub mod build_rwd;
use std::fmt;

pub use build_rwd::*;

pub mod finalize_rwd;
pub use finalize_rwd::*;

pub mod finalize_utxopti;

use super::*;
use bigdecimal::{BigDecimal, ToPrimitive};
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};

// Copy from Hugin / Datamodel
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RewardHandle {
    contract_id: i64,
    stake_addr: String,
    fingerprint: String,
    policy: String,
    tokenname: String,
    tot_earned: u64,
    tot_claimed: u64,
    last_calc_epoch: i64,
}

impl fmt::Display for RewardHandle {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let serde = serde_json::to_string(self)
            .expect("Error: could not convert Rewardhandle to JSON string");
        fmt.write_str(&serde)?;
        Ok(())
    }
}

impl From<String> for RewardHandle {
    fn from(str: String) -> Self {
        let rwd: RewardHandle = serde_json::from_str(&str)
            .expect("Error: could not convert JSON string to Rewardhandle");
        rwd
    }
}

impl RewardHandle {
    pub fn get_policy_id(&self) -> Result<clib::PolicyID, MurinError> {
        clib::PolicyID::from_bytes(hex::decode(&self.policy)?)
            .map_err(|_| MurinError::new("could not get policy-id from reward handle"))
    }
    pub fn get_assetname(&self) -> Result<clib::AssetName, MurinError> {
        clib::AssetName::new(hex::decode(&self.tokenname)?)
            .map_err(|_| MurinError::new("could not get assetname from reward handle"))
    }
    pub fn get_amount(&self) -> Result<clib::utils::BigNum, MurinError> {
        Ok(cutils::BigNum::from(
            (self.tot_earned.to_owned() - self.tot_claimed.to_owned())
                .to_u64()
                .unwrap(),
        ))
    }
    pub fn get_fingerprint(&self) -> String {
        self.fingerprint.clone()
    }
    pub fn get_contract_id(&self) -> i64 {
        self.contract_id
    }
    pub fn get_stake_addr(&self) -> Result<clib::address::RewardAddress, MurinError> {
        let addr = crate::b_decode_addr_na(&self.stake_addr)?;
        clib::address::RewardAddress::from_address(&addr).ok_or_else(|| {
            MurinError::new("Error: could not construct RewardAddress for RewardHandle")
        })
    }
    pub fn get_stake_addr_str(&self) -> String {
        self.stake_addr.clone()
    }

    pub fn total_value(handles: &[RewardHandle]) -> Result<clib::utils::Value, MurinError> {
        let tv: clib::utils::Value =
            handles
                .iter()
                .fold(clib::utils::Value::zero(), |mut acc, n| {
                    let mut assets = clib::Assets::new();
                    assets.insert(&n.get_assetname().unwrap(), &n.get_amount().unwrap());
                    let mut ma = clib::MultiAsset::new();
                    ma.insert(&n.get_policy_id().unwrap(), &assets);
                    let mut v = clib::utils::Value::zero();
                    v.set_multiasset(&ma);
                    acc = acc.checked_add(&v).unwrap();
                    acc
                });
        Ok(tv)
    }

    pub fn value(&self) -> Result<clib::utils::Value, MurinError> {
        let mut assets = clib::Assets::new();
        assets.insert(&self.get_assetname().unwrap(), &self.get_amount().unwrap());
        let mut ma = clib::MultiAsset::new();
        ma.insert(&self.get_policy_id().unwrap(), &assets);
        let mut v = clib::utils::Value::zero();
        v.set_multiasset(&ma);

        Ok(v)
    }
}

#[derive(Debug, Clone)]
pub struct RWDTxData {
    rewards: Vec<RewardHandle>,
    recipient_stake_addr: caddr::Address,
    recipient_payment_addr: caddr::Address,
    fee_wallet_addr: Option<caddr::Address>,
    fee: Option<u64>,
    reward_utxos: Option<TransactionUnspentOutputs>,
}

impl RWDTxData {
    pub fn new(
        rewards: &[RewardHandle],
        recipient_stake_addr: &caddr::Address,
        recipient_payment_addr: &caddr::Address,
        //fee_wallet_addr         : &caddr::Address,
        //fee                     : &u64,
        //reward_utxos            : &Option<TransactionUnspentOutputs>,
    ) -> RWDTxData {
        RWDTxData {
            rewards: rewards.to_vec(),
            recipient_stake_addr: recipient_stake_addr.clone(),
            recipient_payment_addr: recipient_payment_addr.clone(),
            fee_wallet_addr: None, //fee_wallet_addr.clone(),
            fee: None,             //fee,
            reward_utxos: None,    // reward_utxos.clone(),
        }
    }

    pub fn get_rewards(&self) -> Vec<RewardHandle> {
        self.rewards.clone()
    }

    pub fn get_stake_addr(&self) -> caddr::Address {
        self.recipient_stake_addr.clone()
    }

    pub fn get_payment_addr(&self) -> caddr::Address {
        self.recipient_payment_addr.clone()
    }

    pub fn get_fee_wallet_addr(&self) -> Option<caddr::Address> {
        self.fee_wallet_addr.clone()
    }

    pub fn get_fee(&self) -> Option<u64> {
        self.fee
    }

    pub fn get_reward_utxos(&self) -> Option<TransactionUnspentOutputs> {
        self.reward_utxos.clone()
    }

    pub fn set_reward_tokens(&mut self, data: &[RewardHandle]) {
        self.rewards = data.to_vec();
    }

    pub fn set_stake_addr(&mut self, data: &caddr::Address) {
        self.recipient_stake_addr = data.clone();
    }

    pub fn set_payment_addr(&mut self, data: &caddr::Address) {
        self.recipient_payment_addr = data.clone()
    }

    pub fn set_fee_wallet_addr(&mut self, data: &caddr::Address) {
        self.fee_wallet_addr = Some(data.clone());
    }

    pub fn set_fee(&mut self, data: &u64) {
        self.fee = Some(*data);
    }

    pub fn set_nofee(&mut self) {
        self.fee = None;
    }

    pub fn set_reward_utxos(&mut self, data: &Option<TransactionUnspentOutputs>) {
        self.reward_utxos = data.clone();
    }
}

impl ToString for RWDTxData {
    fn to_string(&self) -> String {
        // prepare tokens vector
        let mut s_tokens = String::new();
        for ta in self.get_rewards() {
            //let mut subs = String::new();
            //subs.push_str(&(hex::encode(ta.0.to_bytes()) + "?"));
            //subs.push_str(&(hex::encode(ta.1.to_bytes()) + "?"));
            //subs.push_str(&(hex::encode(ta.2.to_bytes()) + "!"));
            s_tokens.push_str(&ta.to_string());
            s_tokens.push('!');
        }
        // erase last !
        s_tokens.pop();

        // prepare stake address
        let s_stake_addr = hex::encode(self.get_stake_addr().to_bytes());

        // prepare payment address
        let s_payment_addr = hex::encode(self.get_payment_addr().to_bytes());

        // prepare rewards wallet address
        let s_fee_wallet_addr = match self.get_fee_wallet_addr() {
            Some(addr) => hex::encode(addr.to_bytes()),
            None => "NoData".to_string(),
        };

        // prepare fee
        let s_fee = match self.get_fee() {
            Some(fee) => fee.to_string(),
            None => "NoData".to_string(),
        };

        // prepare token_utxos
        let s_token_utxos = match self.get_reward_utxos() {
            Some(u) => {
                if let Ok(s) = u.to_hex() {
                    s
                } else {
                    "NoData".to_string()
                }
            }
            _ => "NoData".to_string(),
        };

        let mut ret = String::new();
        ret.push_str(&(s_tokens + "|"));
        ret.push_str(&(s_stake_addr + "|"));
        ret.push_str(&(s_payment_addr + "|"));
        ret.push_str(&(s_fee_wallet_addr + "|"));
        ret.push_str(&(s_fee + "|"));
        ret.push_str(&(s_token_utxos));

        ret
    }
}

impl core::str::FromStr for RWDTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let slice: Vec<&str> = src.split('|').collect();
        if slice.len() == 6 {
            // restore token vector
            let mut rewards = Vec::<RewardHandle>::new();

            let rwds: Vec<&str> = slice[0].split('!').collect();
            for rwd in rwds {
                rewards.push(RewardHandle::from(rwd.to_string()));
                /*
                let rwd_slice: Vec<&str> = token.split('?').collect();
                tokens.push((
                    clib::PolicyID::from_bytes(hex::decode(token_slice[0])?)?,
                    clib::AssetName::from_bytes(hex::decode(token_slice[1])?)?,
                    cutils::BigNum::from_bytes(hex::decode(token_slice[2])?)?,
                ))
                */
            }
            debug!("Tokens: {:?}", rewards);

            // restore stake address
            let stake_address = caddr::Address::from_bytes(hex::decode(slice[1])?)?;

            // restore payment address
            let payment_address = caddr::Address::from_bytes(hex::decode(slice[2])?)?;

            // restore fee wallet addr
            println!("restore fee wallet addr: {:?}", slice[3]);

            let fee_wallet_addr = match slice[3] {
                "NoData" => None,
                _ => Some(caddr::Address::from_bytes(hex::decode(slice[3])?)?),
            };

            // restore fee
            println!("restore fee");
            let fee = match slice[4] {
                "NoData" => None,
                _ => Some(slice[4].parse::<u64>()?),
            };
            println!("restore token_utxos: {:?}\n\n", slice[5]);
            // restore token_utxos
            let token_utxos = match slice[5] {
                "NoData" => None,
                _ => Some(TransactionUnspentOutputs::from_hex(slice[5])?),
            };
            println!("Restored token utxos");
            Ok(RWDTxData {
                rewards,
                recipient_stake_addr: stake_address,
                recipient_payment_addr: payment_address,
                fee_wallet_addr,
                fee,
                reward_utxos: token_utxos,
            })
        } else {
            Err(MurinError::new(
                //std::io::Error::new(
                //    std::io::ErrorKind::InvalidData,
                &format!(
                    "Error the provided string '{}' cannot be parsed into 'RWDTxData' ",
                    src
                ),
            ))
        }
    }
}
