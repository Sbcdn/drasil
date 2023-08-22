use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct CreateMintProj {
    pub project_name: String,
    pub user_id: Option<i64>,
    pub mint_start_date: Option<DateTime<Utc>>,
    pub mint_end_date: Option<DateTime<Utc>>,
    pub storage_type: String,
    pub storage_url: Option<String>,
    pub storage_access_token: Option<String>,
    pub collection_name: String,
    pub author: String,
    pub meta_description: String,
    pub meta_common_nft_name: Option<String>,
    pub max_mint_p_addr: Option<i32>,
    pub network: u64,
    pub time_constraint: Option<String>,
}

pub enum NftImportType {
    FromMetaDataCSV(),
    FromMetaDataFiles(),
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ImportNFTsfromCSV {
    pub project_id: i64,
    pub csv_hex: String,
}
