pub use crate::error::SleipnirError;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountParams {
    pub contract_id: i64,
    pub user_id: i64,
    pub policy_id: String,
    pub fingerprint: Option<String>,
    pub metadata_path: Vec<String>,
}

pub async fn create_discount(params: DiscountParams) -> Result<serde_json::Value, SleipnirError> {
    let new = drasil_gungnir::Discount::create_discount(
        &params.user_id,
        &params.contract_id,
        &params.policy_id,
        params.fingerprint.as_ref(),
        &params.metadata_path,
    );
    log::debug!("Sleipnir create discount: {:?}", new);
    Ok(json!(new?))
}

pub async fn get_discounts(cid: i64, uid: i64) -> Result<serde_json::Value, SleipnirError> {
    let resp = drasil_gungnir::Discount::get_discounts(cid, uid)?;

    Ok(json!(resp))
}

pub async fn remove_discount(params: DiscountParams) -> Result<serde_json::Value, SleipnirError> {
    let discounts = drasil_gungnir::Discount::get_discounts(params.contract_id, params.user_id)?;
    let discount: Vec<_> = discounts
        .iter()
        .filter(|n| {
            params.policy_id == n.policy_id
                && params.metadata_path == n.metadata_path
                && params.fingerprint == n.fingerprint
        })
        .collect();
    let resp = drasil_gungnir::Discount::remove_discount(&discount[0].id)?;

    Ok(json!(resp))
}
