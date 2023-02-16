/*
#################################################################################
# Business Source License           See LICENSE.md for full license information.#
# Licensor:             Drasil LTD                                              #
# Licensed Work:        Drasil Application Framework v.0.2. The Licensed Work   #
#                       is © 2022 Drasil LTD                                    #
# Additional Use Grant: You may use the Licensed Work when the entity           #
#                       using or operating the Licensed Work is generating      #
#                       less than $150,000 yearly turnover and the entity       #
#                       operating the application engaged less than 10 people.  #
# Change Date:          Drasil Application Framework v.0.2, change date is two  #
#                       and a half years from release date.                     #
# Change License:       Version 2 or later of the GNU General Public License as #
#                       published by the Free Software Foundation.              #
#################################################################################
*/
extern crate pretty_env_logger;

use crate::error::Error;
use sleipnir::jobs::JobTypes;
use sleipnir::models::ImportNFTsfromCSV;
use sleipnir::whitelist::AllocateSpecificAssetsToMintProject;
use sleipnir::whitelist::ImportWhitelistFromCSV;

pub async fn handle_job(job_type: &JobTypes) -> Result<(), Error> {
    match job_type {
        JobTypes::ImportNFTsFromCsv(job) => {
            let data = serde_json::from_value::<ImportNFTsfromCSV>(job.data.clone())?;
            log::debug!("Data {:?}", data);
            sleipnir::minting::api::import_nfts_from_csv_metadata(
                &hex::decode(data.csv_hex).unwrap(),
                job.drasil_user_id,
                data.project_id,
            )
            .await?;
        }
        JobTypes::ImportWhitelist(job) => {
            let data = serde_json::from_value::<ImportWhitelistFromCSV>(job.data.clone())?;
            log::debug!("Data {:?}", data);
            sleipnir::whitelist::import_whitelist_from_csv(
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
            sleipnir::whitelist::allocate_specific_assets_to_mintproject(
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
            sleipnir::whitelist::random_allocation_whitelist_to_mintproject(
                &job.drasil_user_id,
                &data.project_id_in,
                &data.whitelist_id_in,
            )
            .await?;
        }
    }

    Ok(())
}
