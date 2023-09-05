//! Multi signature transaction handlers.
use axum::extract::{Path, State};
use axum::Json;
use drasil_hugin::client::connect;
use drasil_hugin::{
    BuildMultiSig, BuildStdTx, MultiSigType, StdTxType, TXPWrapper, UnsignedTransaction,
};

use crate::error::{Error, Result, TransactionError};
use crate::extractor::Claims;
use crate::state::AppState;

/// Build a multi-signature transactions
#[tracing::instrument(
    name = "build multi-signature-transaction",
    skip(state, claims, payload)
)]
pub async fn build_multi_signature_tx(
    State(state): State<AppState>,
    Path(multisig_type): Path<MultiSigType>,
    claims: Claims,
    Json(payload): Json<TXPWrapper>,
) -> Result<Json<UnsignedTransaction>> {
    match multisig_type {
        MultiSigType::SpoRewardClaim | MultiSigType::NftCollectionMinter | MultiSigType::Mint => {}
        _ => {
            tracing::error!("invalid multisignature type {multisig_type}");
            return Err(TransactionError::Invalid)?;
        }
    }

    let TXPWrapper::TransactionPattern(payload) = payload else {
        return Err(Error::from(TransactionError::Invalid));
    };

    let customer_id = claims.get_customer_id()?;
    tracing::Span::current().record("customer_id", &tracing::field::display(customer_id));
    let cmd = BuildMultiSig::new(customer_id, multisig_type.clone(), *payload.clone());

    tracing::info!("connecting to connect to odin...");
    let mut client = connect(state.odin_url).await?;
    let multi_sig_build_cmd = client
        .build_cmd::<BuildMultiSig>(cmd)
        .await
        .map_err(|err| {
            tracing::error!("failed to build multi-signature transaction command: {err}");
            err
        })?;

    let resp: UnsignedTransaction = match multi_sig_build_cmd.parse() {
        Ok(resp) => resp,
        Err(_) => {
            serde_json::from_str::<UnsignedTransaction>(&multi_sig_build_cmd).map_err(|err| {
                tracing::error!("error could not deserialize Unsigned Transaction: {err}");
                TransactionError::Conflict
            })?
        }
    };

    Ok(Json(resp))
}

/// Build standard transaction.
#[tracing::instrument(name = "Build standard transaction", skip(state, claims))]
pub async fn build_std_tx(
    State(state): State<AppState>,
    Path(tx_type): Path<StdTxType>,
    claims: Claims,
    Json(payload): Json<TXPWrapper>,
) -> Result<Json<UnsignedTransaction>> {
    let TXPWrapper::TransactionPattern(payload) = payload else {
        return Err(Error::from(TransactionError::Invalid));
    };

    let customer_id = claims.get_customer_id()?;
    tracing::Span::current().record("customer_id", &tracing::field::display(customer_id));

    tracing::info!("creating command");
    let cmd = BuildStdTx::new(customer_id, tx_type.clone(), *payload);

    tracing::info!("connecting to connect to odin");
    let mut client = connect(state.odin_url).await?;
    let tx_build_cmd = client.build_cmd::<BuildStdTx>(cmd).await.map_err(|err| {
        tracing::error!("failed to build standard transaction command: {err}");
        err
    })?;

    let resp: UnsignedTransaction = match tx_build_cmd.parse() {
        Ok(resp) => resp,
        Err(_) => serde_json::from_str::<UnsignedTransaction>(&tx_build_cmd).map_err(|err| {
            tracing::error!("could not deserialize Unsigned Transaction: {err}");
            TransactionError::Conflict
        })?,
    };
    Ok(Json(resp))
}
