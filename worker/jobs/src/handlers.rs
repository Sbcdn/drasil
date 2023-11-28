extern crate pretty_env_logger;

use crate::error::Error;
use drasil_sleipnir::jobs::JobTypes;
use drasil_sleipnir::models::ImportNFTsfromCSV;
use drasil_sleipnir::whitelist::AllocateSpecificAssetsToMintProject;
use drasil_sleipnir::whitelist::ImportWhitelistFromCSV;

pub async fn handle_job(job_type: &JobTypes) -> Result<(), Error> {
    match job_type {
        JobTypes::ImportNFTsFromCsv(job) => {
            let data = serde_json::from_value::<ImportNFTsfromCSV>(job.data.clone())?;
            log::debug!("Data {:?}", data);
            drasil_sleipnir::minting::api::import_nfts_from_csv_metadata(
                &hex::decode(data.csv_hex).unwrap(),
                job.drasil_user_id,
                data.project_id,
            )
            .await?;
        }
        JobTypes::ImportWhitelist(job) => {
            let data = serde_json::from_value::<ImportWhitelistFromCSV>(job.data.clone())?;
            log::debug!("Data {:?}", data);
            drasil_sleipnir::whitelist::import_whitelist_from_csv(
                &job.drasil_user_id,
                &data.whitelist_id,
                data.project_id.as_ref(),
                &hex::decode(data.csv).unwrap(),
            )?;
        }
        JobTypes::AllocateSpecificAssetsToMintProject(job) => {
            let data =
                serde_json::from_value::<AllocateSpecificAssetsToMintProject>(job.data.clone())?;
            log::debug!("AllocateSpecificAssetsToMintProject Data {:?}", data);
            drasil_sleipnir::whitelist::allocate_specific_assets_to_mintproject(
                &job.drasil_user_id,
                &data.project_id_in,
                &data.whitelist_id_in,
            )
            .await?;
        }
        JobTypes::RandomAllocateWhitelistToMintProject(job) => {
            let data =
                serde_json::from_value::<AllocateSpecificAssetsToMintProject>(job.data.clone())?;
            log::debug!("RandomAllocateWhitelistToMintProject Data {:?}", data);
            drasil_sleipnir::whitelist::random_allocation_whitelist_to_mintproject(
                &job.drasil_user_id,
                &data.project_id_in,
                &data.whitelist_id_in,
            )
            .await?;
        }
        JobTypes::CalculateReoccuringRewards(_) => todo!(),
        JobTypes::OptimizeRewardUTxOs(_) => todo!(),
    }
    Ok(())
}
