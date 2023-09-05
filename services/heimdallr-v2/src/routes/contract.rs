//! Multi signature transaction handlers.
use anyhow::Context;
use axum::extract::{Path, State};
use axum::Json;
use drasil_hugin::client::connect;
use drasil_hugin::{
    BuildContract, ContractAction, ContractType, FinalizeContract, MarketplaceActions, TXPWrapper,
    TxHash, UnsignedTransaction,
};

use crate::error::{Error, Result, TransactionError};
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
) -> Result<Json<UnsignedTransaction>> {
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

    tracing::info!("building contract command");
    let cmd = BuildContract::new(customer_id, contract, action, *tx_pattern);

    tracing::info!("connecting to odin");
    let mut client = connect(state.odin_url).await?;

    tracing::info!("building contract");
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

/// Finalize contract execution.
pub async fn finalize_contract_exec(
    State(state): State<AppState>,
    Path(contract): Path<ContractType>,
    Path(transaction_id): Path<String>,
    claims: Claims,
    Json(payload): Json<TXPWrapper>,
) -> Result<Json<TxHash>> {
    let TXPWrapper::Signature(payload) = payload else {
        return Err(Error::from(TransactionError::Invalid));
    };

    let customer_id = claims.get_customer_id()?;
    tracing::Span::current().record("customer_id", &tracing::field::display(customer_id));

    tracing::info!("connecting to odin service");
    let mut client = connect(state.odin_url).await?;

    let cmd = FinalizeContract::new(customer_id, contract, transaction_id, payload.get_signature());

    tracing::debug!("finalizing contract");
    let response = client.build_cmd(cmd).await.map_err(|err| {
        tracing::error!("{err}");
        TransactionError::Precondition
    })?;

    Ok(Json(TxHash::new(&response)))
}
