//! Multi signature transaction handlers.
use axum::extract::{Path, State};
use axum::Json;
use drasil_hugin::client::connect;
use drasil_hugin::{
    BuildMultiSig, BuildStdTx, FinalizeMultiSig, FinalizeStdTx, MultiSigType, OneShotReturn,
    StdTxType, TXPWrapper, TransactionPattern, TxHash, UnsignedTransaction,
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

    tracing::info!("connecting to connect to odin");
    let mut client = connect(state.odin_url).await?;

    tracing::info!("building multi-signature transaction command");
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

/// Finalize multi-signatures transactions.
#[tracing::instrument(name = "Finalize multi-signatures transaction", skip(state, claims))]
pub async fn finalize_multi_signature_tx(
    State(state): State<AppState>,
    Path(multisig_type): Path<MultiSigType>,
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

    let cmd = FinalizeMultiSig::new(
        customer_id,
        multisig_type,
        transaction_id,
        payload.get_signature(),
    );
    tracing::debug!("finalizing multi-signatures transaction");
    let response = client.build_cmd(cmd).await.map_err(|err| {
        tracing::error!("{err}");
        TransactionError::Precondition
    })?;

    Ok(Json(TxHash::new(&response)))
}

/// Build standard transaction.
#[tracing::instrument(name = "Build standard transaction", skip(state, claims))]
pub async fn build_standard_tx(
    State(state): State<AppState>,
    Path(transaction_type): Path<StdTxType>,
    claims: Claims,
    Json(payload): Json<TXPWrapper>,
) -> Result<Json<UnsignedTransaction>> {
    let TXPWrapper::TransactionPattern(payload) = payload else {
        return Err(Error::from(TransactionError::Invalid));
    };

    let customer_id = claims.get_customer_id()?;
    tracing::Span::current().record("customer_id", &tracing::field::display(customer_id));

    tracing::info!("building standard transaction command");
    let cmd = BuildStdTx::new(customer_id, transaction_type, *payload);

    tracing::info!("connecting to connect to odin");
    let mut client = connect(state.odin_url).await?;

    tracing::info!("building standard transaction");
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

/// Finalize standard transaction
#[tracing::instrument(name = "Finalize standard transaction", skip(state, claims))]
pub async fn finalize_standard_tx(
    State(state): State<AppState>,
    Path(transaction_type): Path<StdTxType>,
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
    let cmd = FinalizeStdTx::new(
        customer_id,
        transaction_type,
        transaction_id,
        payload.get_signature(),
    );
    tracing::debug!("finalizing transaction");
    let response = client.build_cmd(cmd).await.map_err(|err| {
        tracing::error!("{err}");
        TransactionError::Precondition
    })?;

    Ok(Json(TxHash::new(&response)))
}

/// Build oneshot minter transaction
#[tracing::instrument(name = "Build one shot minter transaction", skip(claims, state))]
pub async fn hnd_oneshot_minter_api(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<TXPWrapper>,
) -> Result<Json<OneShotReturn>> {
    let TXPWrapper::OneShotMinter(payload) = payload else {
        return Err(Error::from(TransactionError::Invalid));
    };

    if payload.tokennames().len() != payload.amounts().len() {
        return Err(Error::from(TransactionError::Invalid));
    }

    let multisig_type = MultiSigType::ClAPIOneShotMint;

    let customer_id = claims.get_customer_id()?;
    tracing::Span::current().record("customer_id", &tracing::field::display(customer_id));

    let transaction_pattern =
        TransactionPattern::new_empty(customer_id, &payload.into_script_spec(), payload.network());

    tracing::info!("connecting to connect to odin");
    let mut client = connect(state.odin_url).await?;

    let cmd = BuildMultiSig::new(customer_id, multisig_type, transaction_pattern);
    let multi_sig_build = client
        .build_cmd::<BuildMultiSig>(cmd)
        .await
        .map_err(|err| {
            tracing::error!("{err}");
            TransactionError::Invalid
        })?;

    let response: OneShotReturn = serde_json::from_str(&multi_sig_build).map_err(|err| {
        tracing::error!("{err}");
        TransactionError::Invalid
    })?;

    Ok(Json(response))
}
