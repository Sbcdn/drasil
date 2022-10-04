/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
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
