pub mod reward_calculation;
pub mod utxo_multiplication;
extern crate pretty_env_logger;

use drasil_murin::MurinError;
use drasil_sleipnir::jobs::JobTypes;
use drasil_sleipnir::models::ImportNFTsfromCSV;
use drasil_sleipnir::whitelist::AllocateSpecificAssetsToMintProject;
use drasil_sleipnir::whitelist::ImportWhitelistFromCSV;

use crate::handlers::reward_calculation::models::CalculateReoccuringRewards;
use crate::handlers::utxo_multiplication::models::OptimizeRewardUTxOs;

use self::reward_calculation::reward_calculation;
use self::utxo_multiplication::run_optimize;

pub async fn handle_job(job_type: &JobTypes) -> Result<(), MurinError> {
    match job_type {
        // Import NFTs from CSV
        JobTypes::ImportNFTsFromCsv(job) => {
            let data = serde_json::from_value::<ImportNFTsfromCSV>(job.data.clone())?;
            log::debug!("Data {:?}", data);
            drasil_sleipnir::minting::api::import_nfts_from_csv_metadata(
                &hex::decode(data.csv_hex).unwrap(),
                job.drasil_user_id,
                data.project_id,
            )
            .await
            .map_err(|e| e.to_string())?;
        }
        // Import a CSV whitelist into the database
        JobTypes::ImportWhitelist(job) => {
            let data = serde_json::from_value::<ImportWhitelistFromCSV>(job.data.clone())?;
            log::debug!("Data {:?}", data);
            drasil_sleipnir::whitelist::import_whitelist_from_csv(
                &job.drasil_user_id,
                &data.whitelist_id,
                data.project_id.as_ref(),
                &hex::decode(data.csv).unwrap(),
            )
            .map_err(|e| e.to_string())?;
        }
        // Allocate NFTs to addresses using a defined whitelist where each NFT has a dedicated address, defined in the whitelist.
        JobTypes::AllocateSpecificAssetsToMintProject(job) => {
            let data =
                serde_json::from_value::<AllocateSpecificAssetsToMintProject>(job.data.clone())?;
            log::debug!("AllocateSpecificAssetsToMintProject Data {:?}", data);
            drasil_sleipnir::whitelist::allocate_specific_assets_to_mintproject(
                &job.drasil_user_id,
                &data.project_id_in,
                &data.whitelist_id_in,
            )
            .await
            .map_err(|e| e.to_string())?;
        }
        // Pseudo-Randomly allocate NFTs to addresses in a Whitelist
        JobTypes::RandomAllocateWhitelistToMintProject(job) => {
            let data =
                serde_json::from_value::<AllocateSpecificAssetsToMintProject>(job.data.clone())?;
            log::debug!("RandomAllocateWhitelistToMintProject Data {:?}", data);
            drasil_sleipnir::whitelist::random_allocation_whitelist_to_mintproject(
                &job.drasil_user_id,
                &data.project_id_in,
                &data.whitelist_id_in,
            )
            .await
            .map_err(|e| e.to_string())?;
        }
        // Calculate Rewards
        JobTypes::CalculateReoccuringRewards(job) => {
            let data = serde_json::from_value::<CalculateReoccuringRewards>(job.data.clone())?;
            log::debug!("CalculateReoccuringRewards Data {:?}", data);
            reward_calculation(data.epoch, data.from)
                .await
                .map_err(|e| e.to_string())?;
        }
        // Multiply the UTxOs on a multi signature native script address
        JobTypes::OptimizeRewardUTxOs(job) => {
            let data = serde_json::from_value::<OptimizeRewardUTxOs>(job.data.clone())?;
            log::debug!("OptimizeRewardUTxOs Data {:?}", data);
            data.ids.iter().for_each(|contract_id| {
                tokio::spawn(run_optimize(*contract_id));
            });
        }
    }
    Ok(())
}
