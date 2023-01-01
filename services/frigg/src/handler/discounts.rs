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
use serde::{Deserialize, Serialize};
use sleipnir::discounts::{create_discount, remove_discount, DiscountParams};
use warp::Reply;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DiscountInput {
    contract_id: i64,
    policy_id: String,
    fingerprint: Option<String>,
    metadata_path: Vec<String>,
}

pub async fn hndl_create_discount(uid: String, input: DiscountInput) -> WebResult<impl Reply> {
    println!("hndl_create_discount");
    let user = get_user_from_string(&uid).await?;
    println!("got user");
    let params = DiscountParams {
        contract_id: input.contract_id,
        user_id: user,
        policy_id: input.policy_id,
        fingerprint: input.fingerprint,
        metadata_path: input.metadata_path,
    };
    println!("try to create discount");
    let discount = create_discount(params).await;
    println!("discount created: {:?}", discount);
    Ok(warp::reply::with_status(
        warp::reply::json(&discount?),
        warp::http::StatusCode::OK,
    ))
}

pub async fn hndl_remove_discount(uid: String, input: DiscountInput) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let params = DiscountParams {
        contract_id: input.contract_id,
        user_id: user,
        policy_id: input.policy_id,
        fingerprint: input.fingerprint,
        metadata_path: input.metadata_path,
    };
    let discount = remove_discount(params).await?;
    Ok(warp::reply::with_status(
        warp::reply::json(&discount),
        warp::http::StatusCode::CREATED,
    ))
}
