use std::convert::Infallible;
use std::env;
use std::str::FromStr;

use drasil_hugin::client::{connect, Client};
use drasil_hugin::datamodel::models::{
    ContractAction, ContractType, MarketplaceActions, MultiSigType, OneShotReturn, ReturnError,
    StdTxType, TransactionPattern, TxHash, UnsignedTransaction,
};
use drasil_hugin::{
    BuildContract, BuildMultiSig, BuildStdTx, FinalizeContract, FinalizeMultiSig, FinalizeStdTx,
    TXPWrapper,
};

use strum::VariantNames;

async fn connect_odin() -> Client {
    connect(env::var("ODIN_URL").unwrap()).await.unwrap()
}

pub async fn contracts_list() -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::json(
        &ContractType::VARIANTS
            .iter()
            .chain(MultiSigType::VARIANTS)
            .collect::<Vec<_>>(),
    ))
}

pub async fn contract_exec_build(
    contract: ContractType,
    action: String,
    (customer_id, payload): (u64, TXPWrapper),
) -> Result<impl warp::Reply, Infallible> {
    let badreq =
        warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::BAD_REQUEST);

    match contract {
        ContractType::MarketPlace => {
            if MarketplaceActions::from_str(&action).is_err() {
                return Ok(badreq);
            }
        }
        ContractType::NftShop => {
            // Not implemented
            return Ok(badreq);
        }
        ContractType::NftMinter => {
            // Not implemented
            return Ok(badreq);
        }
        ContractType::TokenMinter => {
            // Not implemented
            return Ok(badreq);
        }
        _ => {
            // Wrong Parameter
            return Ok(badreq);
        }
    }
    let payload = match payload {
        TXPWrapper::TransactionPattern(txp) => txp,
        _ => return Ok(badreq),
    };
    let mut client = connect_odin().await;
    let action = ContractAction::from_str(&action).unwrap();
    let cmd = BuildContract::new(customer_id, contract.clone(), action, *payload.clone());
    match client.build_cmd::<BuildContract>(cmd).await {
        Ok(ok) => match UnsignedTransaction::from_str(&ok) {
            Ok(resp) => Ok(warp::reply::with_status(
                warp::reply::json(&resp),
                warp::http::StatusCode::OK,
            )),

            Err(_) => match serde_json::from_str::<UnsignedTransaction>(&ok) {
                Ok(resp) => Ok(warp::reply::with_status(
                    warp::reply::json(&resp),
                    warp::http::StatusCode::OK,
                )),

                Err(e) => {
                    log::error!("Error could not deserialize Unsigned Transactions: {}", e);
                    Ok(warp::reply::with_status(
                        warp::reply::json(&ReturnError::new(&e.to_string())),
                        warp::http::StatusCode::CONFLICT,
                    ))
                }
            },
        },
        Err(otherwise) => Ok(warp::reply::with_status(
            warp::reply::json(&ReturnError::new(&otherwise.to_string())),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn multisig_exec_build(
    multisig_type: MultiSigType,
    (customer_id, payload): (u64, TXPWrapper),
) -> Result<impl warp::Reply, Infallible> {
    let badreq =
        warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::BAD_REQUEST);
    log::info!("Build MultiSig Transaction....");
    match multisig_type {
        MultiSigType::SpoRewardClaim => {}
        MultiSigType::NftCollectionMinter => {}
        MultiSigType::NftVendor => {
            return Ok(badreq);
        }
        MultiSigType::Mint => {}
        MultiSigType::TestRewards => {
            return Ok(badreq);
        }
        _ => {
            // Wrong Parameter
            return Ok(badreq);
        }
    }
    let payload = match payload {
        TXPWrapper::TransactionPattern(txp) => txp,
        _ => return Ok(badreq),
    };
    let mut client = connect_odin().await;
    let cmd = BuildMultiSig::new(customer_id, multisig_type.clone(), *payload.clone());
    match client.build_cmd::<BuildMultiSig>(cmd).await {
        Ok(ok) => match UnsignedTransaction::from_str(&ok) {
            Ok(resp) => Ok(warp::reply::with_status(
                warp::reply::json(&resp),
                warp::http::StatusCode::OK,
            )),

            Err(_) => match serde_json::from_str::<UnsignedTransaction>(&ok) {
                Ok(resp) => Ok(warp::reply::with_status(
                    warp::reply::json(&resp),
                    warp::http::StatusCode::OK,
                )),

                Err(e) => {
                    log::error!("Error could not deserialize Unsigned Transaction: {}", e);
                    Ok(warp::reply::with_status(
                        warp::reply::json(&ReturnError::new(&e.to_string())),
                        warp::http::StatusCode::CONFLICT,
                    ))
                }
            },
        },
        Err(otherwise) => Ok(warp::reply::with_status(
            warp::reply::json(&ReturnError::new(&otherwise.to_string())),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn stdtx_exec_build(
    tx_type: StdTxType,
    (customer_id, payload): (u64, TXPWrapper),
) -> Result<impl warp::Reply, Infallible> {
    let badreq =
        warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::BAD_REQUEST);

    match tx_type {
        StdTxType::DelegateStake => {}
        StdTxType::StandardTx => {}
    }
    let payload = match payload {
        TXPWrapper::TransactionPattern(txp) => txp,
        _ => return Ok(badreq),
    };
    log::debug!("Try to connect to odin...");
    let mut client = connect_odin().await;
    log::debug!("Create Command...");
    let cmd = BuildStdTx::new(customer_id, tx_type.clone(), *payload.clone());
    match client.build_cmd::<BuildStdTx>(cmd).await {
        Ok(ok) => match UnsignedTransaction::from_str(&ok) {
            Ok(resp) => Ok(warp::reply::with_status(
                warp::reply::json(&resp),
                warp::http::StatusCode::OK,
            )),

            Err(e1) => match serde_json::from_str::<UnsignedTransaction>(&ok) {
                Ok(resp) => Ok(warp::reply::with_status(
                    warp::reply::json(&resp),
                    warp::http::StatusCode::OK,
                )),

                Err(e) => {
                    log::error!("Error could not deserialize Unsigned Transaction: {}", e);
                    Ok(warp::reply::with_status(
                        warp::reply::json(&ReturnError::new(&e1.to_string())),
                        warp::http::StatusCode::CONFLICT,
                    ))
                }
            },
        },
        Err(otherwise) => Ok(warp::reply::with_status(
            warp::reply::json(&ReturnError::new(&otherwise.to_string())),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn contract_exec_finalize(
    contract: ContractType,
    tx_id: String,
    (customer_id, payload): (u64, TXPWrapper),
) -> Result<impl warp::Reply, Infallible> {
    let badreq =
        warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::BAD_REQUEST);
    let payload = match payload {
        TXPWrapper::Signature(txp) => txp,
        _ => return Ok(badreq),
    };
    let mut client = connect_odin().await;
    let cmd = FinalizeContract::new(
        customer_id,
        contract.clone(),
        tx_id,
        payload.get_signature(),
    );
    let response = match client.build_cmd(cmd).await {
        Ok(res) => warp::reply::with_status(
            warp::reply::json(&TxHash::new(&res)),
            warp::http::StatusCode::OK,
        ),

        Err(e) => warp::reply::with_status(
            warp::reply::json(&ReturnError::new(&e.to_string())),
            warp::http::StatusCode::PRECONDITION_FAILED,
        ),
    };

    Ok(response)
}

pub async fn multisig_exec_finalize(
    multisig_type: MultiSigType,
    tx_id: String,
    (customer_id, payload): (u64, TXPWrapper),
) -> Result<impl warp::Reply, Infallible> {
    let badreq =
        warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::BAD_REQUEST);
    println!("Multisig exec finalize...");
    let payload = match payload {
        TXPWrapper::Signature(txp) => txp,
        _ => return Ok(badreq),
    };
    let mut client = connect_odin().await;
    let cmd = FinalizeMultiSig::new(
        customer_id,
        multisig_type.clone(),
        tx_id,
        payload.get_signature(),
    );
    let response = match client.build_cmd(cmd).await {
        Ok(res) => warp::reply::with_status(
            warp::reply::json(&TxHash::new(&res)),
            warp::http::StatusCode::OK,
        ),

        Err(e) => warp::reply::with_status(
            warp::reply::json(&ReturnError::new(&e.to_string())),
            warp::http::StatusCode::PRECONDITION_FAILED,
        ),
    };

    Ok(response)
}

pub async fn stdtx_exec_finalize(
    txtype: StdTxType,
    tx_id: String,
    (customer_id, payload): (u64, TXPWrapper),
) -> Result<impl warp::Reply, Infallible> {
    let badreq =
        warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::BAD_REQUEST);
    let payload = match payload {
        TXPWrapper::Signature(txp) => txp,
        _ => return Ok(badreq),
    };
    let mut client = connect_odin().await;
    let cmd = FinalizeStdTx::new(customer_id, txtype.clone(), tx_id, payload.get_signature());
    let response = match client.build_cmd(cmd).await {
        Ok(res) => warp::reply::with_status(
            warp::reply::json(&TxHash::new(&res)),
            warp::http::StatusCode::OK,
        ),

        Err(e) => warp::reply::with_status(
            warp::reply::json(&ReturnError::new(&e.to_string())),
            warp::http::StatusCode::PRECONDITION_FAILED,
        ),
    };

    Ok(response)
}

pub async fn hnd_oneshot_minter_api(
    (customer_id, payload): (u64, TXPWrapper),
) -> Result<impl warp::Reply, Infallible> {
    let badreq =
        warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::BAD_REQUEST);
    log::info!("Build Oneshot Minter Transaction....");
    let payload = match payload {
        TXPWrapper::OneShotMinter(p) => p,
        _ => return Ok(badreq),
    };

    if payload.tokennames().len() != payload.amounts().len() {
        return Ok(badreq);
    }

    let multisig_type = MultiSigType::ClAPIOneShotMint;
    let transaction_pattern =
        TransactionPattern::new_empty(customer_id, &payload.into_script_spec(), payload.network());

    let mut client = connect_odin().await;
    let cmd = BuildMultiSig::new(customer_id, multisig_type.clone(), transaction_pattern);
    let response = match client.build_cmd::<BuildMultiSig>(cmd).await {
        Ok(ok) => match serde_json::from_str::<OneShotReturn>(&ok) {
            Ok(resp) => warp::reply::json(&resp),
            Err(e) => warp::reply::json(&ReturnError::new(&e.to_string())),
        },
        Err(otherwise) => warp::reply::json(&ReturnError::new(&otherwise.to_string())),
    };
    Ok(warp::reply::with_status(
        response,
        warp::http::StatusCode::OK,
    ))
}
