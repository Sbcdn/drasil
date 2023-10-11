#![allow(opaque_hidden_inferred_bound)]
use super::handlers;

use drasil_hugin::datamodel::models::{ContractType, MultiSigType, StdTxType, TXPWrapper};
use warp::Filter;

fn api_endpoints() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    resp_option().or(oneshot_minter_api())
}

fn oneshot_minter_api() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
{
    warp::path("api")
        .and(warp::path("mint"))
        .and(warp::path("oneshot"))
        .and(warp::post())
        .and(auth())
        .and_then(handlers::hnd_oneshot_minter_api)
}

pub fn endpoints() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    list_contracts()
        .or(exec_build_multisig())
        .or(exec_build_contract())
        .or(exec_build_stdtx())
        .or(resp_option())
        .or(exec_finalize_contract())
        .or(exec_finalize_multisig())
        .or(exec_finalize_stdtx())
        .or(api_endpoints())
        .or(warp::get().and(warp::any().map(warp::reply)))
}

fn resp_option() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::options()
        .and(warp::header("origin"))
        .map(|origin: String| {
            warp::http::Response::builder()
                .status(warp::http::StatusCode::OK)
                .header("access-control-allow-methods", "HEAD, GET, POST, OPTION")
                .header("access-control-allow-headers", "authorization")
                .header("access-control-allow-credentials", "true")
                .header("access-control-max-age", "300")
                .header("access-control-allow-origin", origin)
                .header("vary", "origin")
                .body("")
        })
}

/// GET contracts
fn list_contracts() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("lcn")
        .and(warp::get())
        .and_then(handlers::contracts_list)
}

/// Build a Smart Contract transaction
fn exec_build_contract() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
{
    warp::path("cn")
        .and(warp::post())
        .and(warp::path::param::<ContractType>())
        .and(warp::path::param::<String>())
        .and(auth())
        .and_then(handlers::contract_exec_build)
}

/// Finalize a Contract transaction
fn exec_finalize_contract(
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("cn")
        .and(warp::path("fn"))
        .and(warp::post())
        .and(warp::path::param::<ContractType>())
        .and(warp::path::param::<String>())
        .and(auth())
        .and_then(handlers::contract_exec_finalize)
}

/// Build a MultiSig transaction
fn exec_build_multisig() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
{
    warp::path("ms")
        .and(warp::post())
        .and(warp::path::param::<MultiSigType>())
        .and(auth())
        .and_then(handlers::multisig_exec_build)
}

/// Finalize a MultiSig transaction
fn exec_finalize_multisig(
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("ms"))
        .and(warp::path("fn"))
        .and(warp::path::param::<MultiSigType>())
        .and(warp::path::param::<String>())
        .and(auth())
        .and_then(handlers::multisig_exec_finalize)
}

/// Build a standard transaction
fn exec_build_stdtx() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("tx")
        .and(warp::post())
        .and(warp::path::param::<StdTxType>())
        .and(auth())
        .and_then(handlers::stdtx_exec_build)
}

/// Build a MultiSig transaction
fn exec_finalize_stdtx() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
{
    warp::path("tx")
        .and(warp::path("fn"))
        .and(warp::post())
        .and(warp::path::param::<StdTxType>())
        .and(warp::path::param::<String>())
        .and(auth())
        .and_then(handlers::stdtx_exec_finalize)
}

fn auth() -> impl Filter<Extract = ((u64, TXPWrapper),), Error = warp::Rejection> + Clone {
    use super::auth::authorize;
    use warp::{
        filters::body::bytes,
        filters::header::headers_cloned,
        http::header::{HeaderMap, HeaderValue},
    };
    headers_cloned()
        .map(move |headers: HeaderMap<HeaderValue>| (headers))
        .and(bytes().map(move |body: bytes::Bytes| (body)))
        .and_then(authorize)
}
