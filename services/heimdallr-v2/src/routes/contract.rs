//! Multi signature transaction handlers.
use axum::extract::{Path, State};
use axum::Json;
use drasil_hugin::client::connect;
use drasil_hugin::{
    BuildContract, ContractAction, ContractType, MarketplaceActions, TXPWrapper,
    UnsignedTransaction,
};

use crate::error::{Error, TransactionError};
use crate::extractor::Claims;
use crate::state::AppState;

/// Build contract.
pub async fn build_contract(
    State(state): State<AppState>,
    Path(contract): Path<ContractType>,
    Path(action): Path<String>,
    claims: Claims,
    Json(payload): Json<TXPWrapper>,
) -> Result<Json<UnsignedTransaction>, Error> {
    let err_resp = Err(Error::TransactionError(TransactionError::Invalid));
    if contract == ContractType::MarketPlace || action.parse::<MarketplaceActions>().is_err() {
        return err_resp;
    }

    let tx_pattern = if let TXPWrapper::TransactionPattern(tx_pattern) = payload {
        tx_pattern
    } else {
        return err_resp;
    };
    let action: ContractAction = action.parse().map_err(|_| TransactionError::Invalid)?;

    let cmd = BuildContract::new(claims.get_customer_id()?, contract, action, *tx_pattern);
    let mut client = connect(state.odin_url).await?;

    let build_contract = client
        .build_cmd::<BuildContract>(cmd)
        .await
        .map_err(|err| {
            // log error
            Error::UnexpectedError(err.to_string())
        })?;

    let resp = match build_contract.parse::<UnsignedTransaction>() {
        Ok(resp) => Json(resp),
        Err(_) => match serde_json::from_str::<UnsignedTransaction>(&build_contract) {
            Ok(resp) => Json(resp),
            Err(_err) => {
                // log::error!("Error could not deserialize Unsigned Transactions: {}", e);
                return Err(Error::TransactionError(TransactionError::Conflict));
            }
        },
    };

    Ok(resp)
}
