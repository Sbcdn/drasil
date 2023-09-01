//! Multi signature transaction handlers.
use axum::extract::{Path, State};
use axum::Json;
use drasil_hugin::client::connect;
use drasil_hugin::{BuildMultiSig, MultiSigType, TXPWrapper, UnsignedTransaction};

use crate::error::{Error, TransactionError};
use crate::extractor::Claims;
use crate::state::AppState;

/// Build a multi-signature transactions
pub async fn build_multi_signature_tx(
    State(state): State<AppState>,
    Path(multisig_type): Path<MultiSigType>,
    claims: Claims,
    Json(payload): Json<TXPWrapper>,
) -> Result<Json<UnsignedTransaction>, Error> {
    match multisig_type {
        MultiSigType::SpoRewardClaim | MultiSigType::NftCollectionMinter | MultiSigType::Mint => {}
        _ => {
            return Err(TransactionError::Invalid)?;
        }
    }

    let payload = match payload {
        TXPWrapper::TransactionPattern(txp) => txp,
        _ => {
            return Err(TransactionError::Invalid)?;
        }
    };

    let customer_id = claims.get_customer_id()?;

    let cmd = BuildMultiSig::new(customer_id, multisig_type.clone(), *payload.clone());

    let mut client = connect(state.odin_url).await?;
    let value = client
        .build_cmd::<BuildMultiSig>(cmd)
        .await
        .map_err(Error::HugginError)?;

    let resp = match value.parse::<UnsignedTransaction>() {
        Ok(resp) => resp,
        Err(_) => serde_json::from_str::<UnsignedTransaction>(&value).map_err(|_| {
            //log::error!("Error could not deserialize Unsigned Transaction: {}", err);
            TransactionError::Conflict
        })?,
    };

    Ok(Json(resp))
}
