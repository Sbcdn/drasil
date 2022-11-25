/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::schema::{mint_projects, mint_rewards, nft_table};
use chrono::serde::ts_seconds::serialize as to_ts;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = mint_projects)]
pub struct MintProject {
    pub id: i64,
    pub project_name: String,
    pub user_id: i64,
    pub mint_contract_id: i64,
    pub whitelists: Option<Vec<i64>>,
    pub mint_start_date: DateTime<Utc>,
    //#[serde(serialize_with = "to_ts")]
    pub mint_end_date: Option<DateTime<Utc>>,
    pub storage_type: String,
    pub storage_url: Option<String>,
    pub storage_access_token: Option<String>,
    pub collection_name: String,
    pub author: String,
    pub meta_description: String,
    pub meta_common_nft_name: Option<String>,
    pub max_mint_p_addr: Option<i32>,
    pub nft_table_name: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = mint_projects)]
pub struct MintProjectNew<'a> {
    pub project_name: &'a String,
    pub user_id: &'a i64,
    pub mint_contract_id: &'a i64,
    pub whitelists: Option<&'a Vec<i64>>,
    pub mint_start_date: &'a DateTime<Utc>,
    pub mint_end_date: Option<&'a DateTime<Utc>>,
    pub storage_type: &'a String,
    pub storage_url: Option<&'a String>,
    pub storage_access_token: Option<&'a String>,
    pub collection_name: &'a String,
    pub author: &'a String,
    pub meta_description: &'a String,
    pub meta_common_nft_name: Option<&'a String>,
    pub max_mint_p_addr: Option<&'a i32>,
    pub nft_table_name: &'a String,
    pub active: &'a bool,
}

#[derive(Queryable, Debug, Clone, Serialize, Deserialize, QueryableByName)]
pub struct UnknownResponse {}

#[derive(
    Queryable, Identifiable, PartialEq, Eq, Debug, Clone, Serialize, Deserialize, QueryableByName,
)]
#[diesel(table_name = nft_table)]
#[diesel(primary_key(project_id, asset_name_b))]
pub struct Nft {
    pub project_id: i64,
    pub asset_name_b: Vec<u8>,
    pub asset_name: String,
    pub fingerprint: String,
    pub nft_id: String,
    pub file_name: Option<String>,
    pub ipfs_hash: Option<String>,
    pub metadata: Option<String>,
    pub claim_addr: Option<String>,
    pub minted: bool,
    pub tx_hash: Option<String>,
    pub confirmed: bool,
    #[serde(serialize_with = "to_ts")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "to_ts")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = nft_table)]
pub struct NftNew<'a> {
    pub project_id: &'a i64,
    pub asset_name_b: &'a Vec<u8>,
    pub asset_name: &'a String,
    pub fingerprint: &'a String,
    pub nft_id: &'a String,
    pub file_name: Option<&'a String>,
    pub ipfs_hash: Option<&'a String>,
    pub metadata: Option<&'a String>,
    pub claim_addr: Option<&'a String>,
    pub minted: &'a bool,
    pub tx_hash: Option<&'a String>,
    pub confirmed: &'a bool,
}

#[derive(Queryable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = mint_rewards)]
pub struct MintReward {
    pub id: i64,
    pub project_id: i64,
    pub pay_addr: String,
    pub nft_ids: Vec<Vec<u8>>,
    pub v_nfts_b: Vec<Vec<u8>>, // serialized clib::utils::Value
    pub processed: bool,
    pub minted: bool,
    #[serde(serialize_with = "to_ts")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "to_ts")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = mint_rewards)]
pub struct MintRewardNew<'a> {
    pub project_id: &'a i64,
    pub pay_addr: &'a String,
    pub nft_ids: Vec<&'a Vec<u8>>,
    pub v_nfts_b: Vec<&'a Vec<u8>>, // serialized clib::utils::Value
    pub processed: &'a bool,
    pub minted: &'a bool,
}
