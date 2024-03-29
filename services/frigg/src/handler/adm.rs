use super::get_user_from_string;
use crate::WebResult;
use serde::{Deserialize, Serialize};
use serde_json::json;
use warp::Reply;

#[derive(Serialize, Deserialize, Debug)]
pub struct CrLqdtContr {
    network: u8,
}

pub async fn adm_create_lqdt(uid: String, cparam: CrLqdtContr) -> WebResult<impl Reply> {
    let mut net = drasil_murin::clib::NetworkIdKind::Mainnet;
    if cparam.network == 0 {
        net = drasil_murin::clib::NetworkIdKind::Testnet;
    }

    let user = get_user_from_string(&uid).await?;

    let addr = drasil_sleipnir::administration::create_lqdt_wallet(net, &user).await?;
    Ok(serde_json::json!({ "address": addr }).to_string())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrPayout {
    contract_id: i64,
    ada: i64,
    token: Vec<drasil_hugin::Token>,
    pw: String,
}

pub async fn adm_create_payout(uid: String, cparam: CrPayout) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let payout = drasil_sleipnir::user::create_custom_payout(
        user,
        cparam.contract_id,
        cparam.ada,
        cparam.token,
        cparam.pw,
    )
    .await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "created": payout })),
        warp::http::StatusCode::CREATED,
    ))
}

#[derive(Deserialize, Debug, Clone)]
pub struct ExPayout {
    pub po_id: i64,
    pub pw: String,
}

pub async fn adm_execute_payout(uid: String, cparam: ExPayout) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;
    log::debug!("Try to find payout...");
    let payout = drasil_sleipnir::user::find_payout(user, cparam.po_id).await?;

    log::debug!("Try to execute...");
    match payout.execute(&cparam.pw).await {
        Ok(o) => Ok(warp::reply::with_status(
            warp::reply::json(&json!({ "success": o })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => Ok(warp::reply::with_status(
            warp::reply::json(&json!({ "failed": e.to_string() })),
            warp::http::StatusCode::PRECONDITION_FAILED,
        )),
    }
}

pub async fn adm_list_payouts(uid: String) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let payout = drasil_sleipnir::user::show_payouts(user).await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&json!(payout)),
        warp::http::StatusCode::OK,
    ))
}
