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
use serde::Serialize;

use crate::error::RWDError;

use bigdecimal::BigDecimal;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env; //ToPrimitive
              //use diesel_derive_enum::*;

use chrono::serde::ts_seconds::serialize as to_ts;
use chrono::{DateTime, Utc};
use std::fmt;

use crate::schema::{
    airdrop_parameter, airdrop_whitelist, claimed, mint_projects, nft_table, rewards,
    token_whitelist, whitelist, wladdresses, wlalloc,
};

pub fn establish_connection() -> Result<PgConnection, RWDError> {
    dotenv().ok();
    Ok(PgConnection::establish(&env::var("REWARDS_DB_URL")?)?)
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    SqlType,
)] //FromSqlRow DbEnum
#[sql_type = "Calculationmode"]
#[postgres(type_name = "Calculationmode")]
pub enum Calculationmode {
    Custom,
    ModifactorAndEquation,
    SimpleEquation,
    FixedEndEpoch,
    RelationalToADAStake,
    AirDrop,
}

impl diesel::serialize::ToSql<Calculationmode, diesel::pg::Pg> for Calculationmode {
    fn to_sql<W: std::io::Write>(
        &self,
        out: &mut diesel::serialize::Output<W, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        match *self {
            Calculationmode::Custom => out.write_all(b"custom")?,
            Calculationmode::ModifactorAndEquation => out.write_all(b"modifactorandequation")?,
            Calculationmode::SimpleEquation => out.write_all(b"simpleequation")?,
            Calculationmode::FixedEndEpoch => out.write_all(b"fixedendepoch")?,
            Calculationmode::RelationalToADAStake => out.write_all(b"relationaltoadastake")?,
            Calculationmode::AirDrop => out.write_all(b"airdrop")?,
        }
        Ok(diesel::serialize::IsNull::No)
    }
}
impl diesel::deserialize::FromSql<Calculationmode, diesel::pg::Pg> for Calculationmode {
    fn from_sql(bytes: Option<&[u8]>) -> diesel::deserialize::Result<Self> {
        match not_none!(bytes) {
            b"custom" => Ok(Calculationmode::Custom),
            b"modifactorandequation" => Ok(Calculationmode::ModifactorAndEquation),
            b"simpleequation" => Ok(Calculationmode::SimpleEquation),
            b"fixedendepoch" => Ok(Calculationmode::FixedEndEpoch),
            b"relationaltoadastake" => Ok(Calculationmode::RelationalToADAStake),
            b"airdrop" => Ok(Calculationmode::AirDrop),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
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

#[derive(
    Queryable, Identifiable, PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize,
)]
#[table_name = "rewards"]
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

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "rewards"]
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

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone)]
#[table_name = "claimed"]
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

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "claimed"]
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

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone, serde::Serialize)]
#[table_name = "token_whitelist"]
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

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "token_whitelist"]
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

#[derive(Queryable, PartialEq, Debug, Clone, Serialize)]
pub struct TokenInfo {
    pub policy: String,
    pub tokenname: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone)]
#[table_name = "airdrop_whitelist"]
pub struct AirDropWhitelist {
    pub id: i64,
    pub contract_id: i64,
    pub user_id: i64,
    pub reward_created: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "airdrop_whitelist"]
pub struct AirDropWhitelistNew<'a> {
    pub contract_id: &'a i64,
    pub user_id: &'a i64,
    pub reward_created: &'a bool,
}

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone)]
#[table_name = "airdrop_parameter"]
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

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "airdrop_parameter"]
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

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone)]
#[table_name = "wladdresses"]
pub struct WlAddresses {
    pub id: i64,
    pub payment_address: String,
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "wladdresses"]
pub struct WlAddressesNew<'a> {
    pub payment_address: &'a String,
}

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone)]
#[primary_key(wl, addr)]
#[table_name = "wlalloc"]
pub struct WlAlloc {
    pub wl: i64,
    pub addr: i64,
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "wlalloc"]
pub struct WlAllocNew<'a> {
    pub wl: &'a i64,
    pub addr: &'a i64,
}

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone)]
#[table_name = "whitelist"]
pub struct Whitelist {
    pub id: i64,
    pub max_addr_repeat: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "whitelist"]
pub struct WhitelistNew<'a> {
    pub max_addr_repeat: &'a i32,
}

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone)]
#[table_name = "mint_projects"]
pub struct MintProject {
    pub id: i64,
    pub customer_name: String,
    pub project_name: String,
    pub user_id: i64,
    pub contract_id: i64,
    pub whitelist_id: Option<i64>,
    pub mint_start_date: DateTime<Utc>,
    pub mint_end_date: Option<DateTime<Utc>>,
    pub storage_folder: String,
    pub max_trait_count: i32,
    pub collection_name: String,
    pub author: String,
    pub meta_description: String,
    pub max_mint_p_addr: Option<i32>,
    pub reward_minter: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "mint_projects"]
pub struct MintProjectNew<'a> {
    pub customer_name: &'a String,
    pub project_name: &'a String,
    pub user_id: &'a i64,
    pub contract_id: &'a i64,
    pub whitelist_id: Option<&'a i64>,
    pub mint_start_date: &'a DateTime<Utc>,
    pub mint_end_date: Option<&'a DateTime<Utc>>,
    pub storage_folder: &'a String,
    pub max_trait_count: &'a i32,
    pub collection_name: &'a String,
    pub author: &'a String,
    pub meta_description: &'a String,
    pub max_mint_p_addr: Option<&'a i32>,
    pub reward_minter: &'a bool,
}

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone)]
#[table_name = "nft_table"]
#[primary_key(project_id, asset_name_b)]
pub struct Nft {
    pub project_id: i64,
    pub asset_name_b: Vec<u8>,
    pub asset_name: String,
    pub picture_id: String,
    pub file_name: String,
    pub ipfs_hash: Option<String>,
    pub trait_category: Vec<String>,
    pub traits: Vec<Vec<String>>,
    pub metadata: String,
    pub payment_addr: Option<String>,
    pub minted: bool,
    pub tx_hash: Option<String>,
    pub confirmed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name = "nft_table"]
pub struct NftNew<'a> {
    pub project_id: &'a i64,
    pub asset_name_b: &'a Vec<u8>,
    pub asset_name: &'a String,
    pub picture_id: &'a String,
    pub file_name: &'a String,
    pub ipfs_hash: Option<&'a String>,
    pub trait_category: &'a Vec<String>,
    pub traits: &'a Vec<Vec<String>>,
    pub metadata: &'a String,
    pub payment_addr: Option<&'a String>,
    pub minted: &'a bool,
    pub tx_hash: Option<&'a String>,
    pub confirmed: &'a bool,
}
