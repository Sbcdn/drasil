//! # Routes

mod contract;
mod transaction;

use axum::routing::{get, post};
use axum::{Json, Router};
use drasil_hugin::{ContractType, MultiSigType};
use strum::VariantNames;

use crate::state::AppState;

/// List the contracts and multi signature types.
pub async fn list_contracts() -> Json<Vec<&'static str>> {
    let contracts = ContractType::VARIANTS
        .iter()
        .chain(MultiSigType::VARIANTS.iter())
        .cloned()
        .collect();

    Json(contracts)
}

/// Register handlers.
pub fn register_handlers(state: AppState) -> Router {
    Router::new()
        .route("/lcn", get(list_contracts))
        .route(
            "/ms/:multisig_type",
            post(transaction::build_multi_signature_tx),
        )
        .route(
            "/ms/fn/:multisig_type/:transaction_id",
            post(transaction::finalize_multi_signature_tx),
        )
        .route(
            "/tx/:transaction_type",
            post(transaction::build_standard_tx),
        )
        .route(
            "/fn/:transaction_type/:transaction_id",
            post(transaction::finalize_standard_tx),
        )
        .route("/cn/:contract/:action", post(contract::build_contract))
        .route(
            "/fn/:contract/:transaction_id",
            post(contract::finalize_contract_exec),
        )
        .route("api/mint/onshot", post(transaction::hnd_oneshot_minter_api))
        .with_state(state)
}
