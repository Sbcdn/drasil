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
use serde::Deserialize;
use serde_json::json;
use warp::Reply;

#[derive(Deserialize, Debug, Clone)]
pub struct CrLqdtContr {
    network: u8,
}

pub async fn adm_create_lqdt(uid: String, cparam: CrLqdtContr) -> WebResult<impl Reply> {
    let mut net = murin::clib::NetworkIdKind::Mainnet;
    if cparam.network == 0 {
        net = murin::clib::NetworkIdKind::Testnet;
    }

    let user = get_user_from_string(&uid).await?;

    let addr = sleipnir::administration::create_lqdt_wallet(net, &user).await?;
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "address": addr })),
        warp::http::StatusCode::CREATED,
    ))
}
