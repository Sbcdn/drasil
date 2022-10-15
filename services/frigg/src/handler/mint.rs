/*
#################################################################################
# Business Source License           See LICENSE.md for full license information.#
# Licensor:             Drasil Blockchain Association                           #
# Licensed Work:        Drasil Application Framework v.0.2. The Licensed Work   #
#                       is © 2022 Drasil Blockchain Association                 #
# Additional Use Grant: You may use the Licensed Work when your application     #
#                       using the Licensed Work is generating less than         #
#                       $150,000 and the entity operating the application       #
#                       engaged equal or less than 10 people.                   #
# Change Date:          Drasil Application Framework v.0.2, change date is two  #
#                       and a half years from release date.                     #
# Change License:       Version 2 or later of the GNU General Public License as #
#                       published by the Free Software Foundation.              #
#################################################################################
*/
use super::get_user_from_string;
use crate::WebResult;
use serde_json::json;
use sleipnir::models::{CreateMintProj, ImportNFTsfromCSV};
use warp::Reply;

pub async fn entrp_create_mint_proj(uid: String, param: CreateMintProj) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;
    let mut param = param.clone();
    param.user_id = Some(user);

    let contract_id = sleipnir::minting::api::create_mintproject(&param).await?;
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "contract_id": contract_id })),
        warp::http::StatusCode::CREATED,
    ))
}

pub async fn entrp_create_nfts_from_csv(
    uid: String,
    params: ImportNFTsfromCSV,
) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let i = sleipnir::minting::api::import_nfts_from_csv_metadata(
        &hex::decode(params.csv_hex).unwrap(),
        user,
        params.project_id,
    )
    .await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "imported": i })),
        warp::http::StatusCode::CREATED,
    ))
}
