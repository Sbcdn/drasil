use super::get_user_from_string;
use crate::WebResult;
use drasil_sleipnir::discounts::{create_discount, remove_discount, DiscountParams};
use serde::{Deserialize, Serialize};
use warp::Reply;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DiscountInput {
    contract_id: i64,
    policy_id: String,
    fingerprint: Option<String>,
    metadata_path: Vec<String>,
}

pub async fn hndl_create_discount(uid: String, input: DiscountInput) -> WebResult<impl Reply> {
    log::debug!("hndl_create_discount");
    let user = get_user_from_string(&uid).await?;
    log::debug!("got user");
    let params = DiscountParams {
        contract_id: input.contract_id,
        user_id: user,
        policy_id: input.policy_id,
        fingerprint: input.fingerprint,
        metadata_path: input.metadata_path,
    };
    log::debug!("try to create discount");
    let discount = create_discount(params).await;
    log::debug!("discount created: {:?}", discount);
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
