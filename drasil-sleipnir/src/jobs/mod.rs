use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Job {
    pub drasil_user_id: i64,
    pub session_id: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum JobTypes {
    ImportNFTsFromCsv(Job),
    ImportWhitelist(Job),
    AllocateSpecificAssetsToMintProject(Job),
    RandomAllocateWhitelistToMintProject(Job),
    CalculateReoccuringRewards(Job),
    OptimizeRewardUTxOs(Job),
}
