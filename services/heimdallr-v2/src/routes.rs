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
        .route("/cn/:contract/:action", post(contract::build_contract))
        .route("/tx/:tx_type", post(transaction::build_std_tx))
        .with_state(state)
}
