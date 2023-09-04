//! Multi signature transaction handlers.
use anyhow::Context;
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
#[tracing::instrument(name = "Build smart contract", skip(state, claims))]
pub async fn build_contract(
    State(state): State<AppState>,
    Path(contract): Path<ContractType>,
    Path(action): Path<String>,
    claims: Claims,
    Json(payload): Json<TXPWrapper>,
) -> Result<Json<UnsignedTransaction>, Error> {
    if contract == ContractType::MarketPlace {
        if let Err(err) = action
            .parse::<MarketplaceActions>()
            .context("failed to parse marketplace actions")
        {
            tracing::error!("failed to parse marketplace actions {err}");
            return Err(Error::from(err));
        }
    }

    let TXPWrapper::TransactionPattern(tx_pattern) = payload else {
        return Err(Error::from(TransactionError::Invalid));
    };
    let action: ContractAction = action.parse().map_err(|_| TransactionError::Invalid)?;

    let customer_id = claims.get_customer_id()?;
    tracing::Span::current().record("customer_id", &tracing::field::display(customer_id));
    let cmd = BuildContract::new(customer_id, contract, action, *tx_pattern);
    let mut client = connect(state.odin_url).await?;

    let build_contract = client
        .build_cmd::<BuildContract>(cmd)
        .await
        .map_err(|err| {
            tracing::error!("failed to build contract {err}");
            err
        })?;

    let resp = match build_contract.parse::<UnsignedTransaction>() {
        Ok(resp) => Json(resp),
        Err(_) => match serde_json::from_str::<UnsignedTransaction>(&build_contract) {
            Ok(resp) => Json(resp),
            Err(err) => {
                tracing::error!("error could not deserialize Unsigned Transactions: {err}");
                return Err(Error::from(TransactionError::Conflict));
            }
        },
    };

    Ok(resp)
}
