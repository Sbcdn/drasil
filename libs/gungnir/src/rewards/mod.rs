/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
#![allow(clippy::extra_unused_lifetimes)]

pub mod api;
pub use api::*;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::error::RWDError;

use bigdecimal::BigDecimal;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel_derive_enum::*;
use std::env;

use chrono::serde::ts_seconds::serialize as to_ts;
use chrono::{DateTime, Utc};
use std::fmt;

use crate::schema::{
    airdrop_parameter, airdrop_whitelist, claimed, discount, rewards, token_whitelist, whitelist,
    wladdresses, wlalloc,
};

pub fn establish_connection() -> Result<PgConnection, RWDError> {
    Ok(PgConnection::establish(&env::var("REWARDS_DB_URL")?)?)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::Calculationmode"]
pub enum Calculationmode {
    #[db_rename = "custom"]
    Custom,
    #[db_rename = "modifactorandequation"]
    ModifactorAndEquation,
    #[db_rename = "simpleequation"]
    SimpleEquation,
    #[db_rename = "fixedendepoch"]
    FixedEndEpoch,
    #[db_rename = "relationaltoadastake"]
    RelationalToADAStake,
    #[db_rename = "airdrop"]
    AirDrop,
}

impl ToString for Calculationmode {
    fn to_string(&self) -> String {
        match self {
            Self::Custom => "custom".to_string(),
            Self::ModifactorAndEquation => "modifactorandequation".to_string(),
            Self::SimpleEquation => "simpleequation".to_string(),
            Self::FixedEndEpoch => "fixedendepoch".to_string(),
            Self::RelationalToADAStake => "relationaltoadastake".to_string(),
            Self::AirDrop => "airdrop".to_string(),
        }
    }
}

impl std::str::FromStr for Calculationmode {
    type Err = RWDError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "custom" => Ok(Calculationmode::Custom),
            "modifactorandequation" => Ok(Calculationmode::ModifactorAndEquation),
            "simpleequation" => Ok(Calculationmode::SimpleEquation),
            "fixedendepoch" => Ok(Calculationmode::FixedEndEpoch),
            "relationaltoadastake" => Ok(Calculationmode::RelationalToADAStake),
            "airdrop" => Ok(Calculationmode::AirDrop),
            _ => Err(RWDError::new(&format!(
                "Calculationmode {} does not exist",
                src
            ))),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct GPools {
    pub pool_id: String,
    pub first_valid_epoch: i64,
}

impl fmt::Display for GPools {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        //let mut str = "";
        fmt.write_str(&self.pool_id.to_string())?;
        fmt.write_str(",")?;
        fmt.write_str(&self.first_valid_epoch.to_string())?;
        Ok(())
    }
}

impl std::str::FromStr for GPools {
    type Err = RWDError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = src.split(',').collect();
        Ok(GPools {
            pool_id: split[0].to_string(),
            first_valid_epoch: split[1].parse::<i64>()?,
        })
    }
}

impl PartialEq for GPools {
    fn eq(&self, other: &Self) -> bool {
        self.pool_id == other.pool_id
    }
}

#[derive(Queryable, Identifiable, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = rewards)]
pub struct Rewards {
    pub id: i64,
    pub stake_addr: String,
    pub payment_addr: String,
    pub fingerprint: String,
    pub contract_id: i64,
    pub user_id: i64,
    pub tot_earned: BigDecimal,
    pub tot_claimed: BigDecimal,
    pub oneshot: bool,
    pub last_calc_epoch: i64,
    #[serde(serialize_with = "to_ts")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "to_ts")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = rewards)]
pub struct RewardsNew<'a> {
    pub stake_addr: &'a String,
    pub payment_addr: &'a String,
    pub fingerprint: &'a String,
    pub contract_id: &'a i64,
    pub user_id: &'a i64,
    pub tot_earned: &'a BigDecimal,
    pub tot_claimed: &'a BigDecimal,
    pub oneshot: &'a bool,
    pub last_calc_epoch: &'a i64,
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = claimed)]
pub struct Claimed {
    pub id: i64,
    pub stake_addr: String,
    pub payment_addr: String,
    pub fingerprint: String,
    pub amount: BigDecimal,
    pub contract_id: i64,
    pub user_id: i64,
    pub txhash: String,
    pub invalid: Option<bool>,
    pub invalid_descr: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = claimed)]
pub struct ClaimedNew<'a> {
    pub stake_addr: &'a String,
    pub payment_addr: &'a String,
    pub fingerprint: &'a String,
    pub amount: &'a BigDecimal,
    pub contract_id: &'a i64,
    pub user_id: &'a i64,
    pub txhash: &'a String,
    pub invalid: Option<&'a bool>,
    pub invalid_descr: Option<&'a String>,
}

#[derive(Queryable, Identifiable, Debug, Clone, serde::Serialize)]
#[diesel(table_name = token_whitelist)]
pub struct TokenWhitelist {
    pub id: i64,
    pub fingerprint: Option<String>,
    pub policy_id: String,
    pub tokenname: Option<String>,
    pub contract_id: i64,
    pub user_id: i64,
    pub vesting_period: DateTime<Utc>,
    pub pools: Vec<String>,
    pub mode: Calculationmode,
    pub equation: String,
    pub start_epoch: i64,
    pub end_epoch: Option<i64>,
    pub modificator_equ: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = token_whitelist)]
pub struct TokenWhitelistNew<'a> {
    pub fingerprint: &'a String,
    pub policy_id: &'a String,
    pub tokenname: &'a String,
    pub contract_id: &'a i64,
    pub user_id: &'a i64,
    pub vesting_period: &'a DateTime<Utc>,
    pub pools: &'a Vec<String>,
    pub mode: &'a Calculationmode,
    pub equation: &'a String,
    pub start_epoch: &'a i64,
    pub end_epoch: Option<&'a i64>,
    pub modificator_equ: Option<&'a String>,
}

#[derive(Queryable, Debug, Clone, Serialize)]
pub struct TokenInfo {
    pub policy: String,
    pub tokenname: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = airdrop_whitelist)]
pub struct AirDropWhitelist {
    pub id: i64,
    pub contract_id: i64,
    pub user_id: i64,
    pub reward_created: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = airdrop_whitelist)]
pub struct AirDropWhitelistNew<'a> {
    pub contract_id: &'a i64,
    pub user_id: &'a i64,
    pub reward_created: &'a bool,
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = airdrop_parameter)]
pub struct AirDropParameter {
    pub id: i64,
    pub contract_id: i64,
    pub user_id: i64,
    pub airdrop_token_type: String,
    pub distribution_type: String,
    pub selection_type: String,
    pub args_1: Vec<String>,
    pub args_2: Vec<String>,
    pub args_3: Vec<String>,
    pub whitelist_ids: Option<Vec<i64>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = airdrop_parameter)]
pub struct AirDropParameterNew<'a> {
    pub contract_id: &'a i64,
    pub user_id: &'a i64,
    pub airdrop_token_type: &'a String,
    pub distribution_type: &'a String,
    pub selection_type: &'a String,
    pub args_1: &'a Vec<String>,
    pub args_2: &'a Vec<String>,
    pub args_3: &'a Vec<String>,
    pub whitelist_ids: Option<&'a Vec<i64>>,
}

#[derive(Queryable, Debug, Clone)]
// return type for a single whitelist entry for a project
pub struct WlEntry {
    pub id: i64,
    pub payment_address: String,
    pub stake_address: Option<String>,
    pub wl: i64,
    pub alloc_id: i64,
    pub specific_asset: Option<serde_json::Value>, //Specific Asset
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = wladdresses)]
pub struct WlAddresses {
    pub id: i64,
    pub payment_address: String,
    pub stake_address: Option<String>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = wladdresses)]
pub struct WlAddressesNew<'a> {
    pub payment_address: &'a String,
    pub stake_address: Option<&'a String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SpecificAsset {
    pub project_id: i64,
    pub assetname_b: String,
    pub fingerprint: String,
    pub amount: u64,
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(primary_key(wl, addr))]
#[diesel(table_name = wlalloc)]
pub struct WlAlloc {
    pub wl: i64,
    pub addr: i64,
    pub specific_asset: Option<serde_json::Value>, //Specific Asset
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = wlalloc)]
pub struct WlAllocNew<'a> {
    pub wl: &'a i64,
    pub addr: &'a i64,
    pub specific_asset: Option<&'a serde_json::Value>, //Specific Asset
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, DbEnum, Display)]
#[ExistingTypePath = "crate::schema::sql_types::WhitelistType"]
pub enum WhitelistType {
    // A legit user is contained in the whitelist and limited by max mints per user, not contained users cannot mint
    #[db_rename = "RandomContained"]
    RandomContained,
    // A user is allowed to mint a preconfigured specific asset. No unspecified mints.
    #[db_rename = "SpecificAsset"]
    SpecificAsset,
    // Users are preallocated randomly to a token from a whitelist (mix of RandomContained and SpecificAsset)
    #[db_rename = "RandomPreallocated"]
    RandomPreallocated,
}

#[derive(Queryable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = whitelist)]
pub struct Whitelist {
    pub id: i64,
    pub user_id: i64,
    pub max_addr_repeat: i32,
    pub wl_type: WhitelistType,
    pub description: String,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = whitelist)]
pub struct WhitelistNew<'a> {
    pub user_id: &'a i64,
    pub max_addr_repeat: &'a i32,
    pub wl_type: &'a WhitelistType,
    pub description: &'a String,
    pub notes: &'a String,
}

#[derive(Queryable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = discount)]
pub struct Discount {
    pub id: i64,
    pub contract_id: i64,
    pub user_id: i64,
    pub policy_id: String,
    pub fingerprint: Option<String>,
    pub metadata_path: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = discount)]
pub struct DiscountNew<'a> {
    pub contract_id: &'a i64,
    pub user_id: &'a i64,
    pub policy_id: &'a String,
    pub fingerprint: Option<&'a String>,
    pub metadata_path: &'a Vec<String>,
}
