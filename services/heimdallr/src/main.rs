#![allow(opaque_hidden_inferred_bound)]
extern crate pretty_env_logger;

mod clientapi;
mod error;
use std::env;
use std::str;
use warp::Filter;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "4000";

#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }

    let host: String = env::var("POD_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port = env::var("POD_PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string());

    pretty_env_logger::init();

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "OPTIONS", "PUT"])
        .allow_credentials(true)
        .allow_headers(vec![
            "Access-Control-Allow-Origin",
            "Access-Control-Allow-Credentials",
            "Access-Control-Allow-Headers",
            "Access-Control-Allow-Methods",
            "Access-Control-Expose-Headers",
            "Access-Control-Max-Age",
            "Access-Control-Request-Headers",
            "Access-Control-Request-Method",
            "Origin",
            "XMLHttpRequest",
            "X-Requested-With",
            "Accept",
            "Content-Type",
            "Referer",
            "User-Agent",
            "sec-ch-ua",
            "sec-ch-ua-mobile",
            "sec-ch-ua-platform",
            "Accept-Encoding",
            "Accept-Language",
            "authorization",
            "Connection",
            "Content-Length",
            "Host",
            "Sec-Fetch-Dest",
            "Sec-Fetch-Mode",
            "Sec-Fetch-Site",
        ]);

    let api = filters::endpoints();
    let routes = api.with(cors).with(warp::log("heimdallr"));
    let server = host.clone() + ":" + &port;
    let socket: std::net::SocketAddr = server.parse().expect("Unable to parse socket address");
    warp::serve(routes).run(socket).await;
}

///Filters
mod filters {
    use super::handlers;
    use crate::clientapi::filter::api_endpoints;
    use hugin::datamodel::models::{ContractType, MultiSigType, StdTxType, TXPWrapper};
    use warp::Filter;

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

    pub fn resp_option() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        warp::options()
            .and(warp::header("origin"))
            .map(|origin: String| {
                Ok(warp::http::Response::builder()
                    .status(warp::http::StatusCode::OK)
                    .header("access-control-allow-methods", "HEAD, GET, POST, OPTION")
                    .header("access-control-allow-headers", "authorization")
                    .header("access-control-allow-credentials", "true")
                    .header("access-control-max-age", "300")
                    .header("access-control-allow-origin", origin)
                    .header("vary", "origin")
                    .body(""))
            })
    }

    /// GET contracts
    pub fn list_contracts(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("lcn")
            .and(warp::get())
            .and_then(handlers::contracts_list)
    }

    /// Build a Smart Contract transaction
    pub fn exec_build_contract(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("cn")
            .and(warp::post())
            .and(warp::path::param::<ContractType>())
            .and(warp::path::param::<String>())
            .and(auth())
            .and_then(handlers::contract_exec_build)
    }

    /// Finalize a Contract transaction
    pub fn exec_finalize_contract(
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
    pub fn exec_build_multisig(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("ms")
            .and(warp::post())
            .and(warp::path::param::<MultiSigType>())
            .and(auth())
            .and_then(handlers::multisig_exec_build)
    }

    /// Finalize a MultiSig transaction
    pub fn exec_finalize_multisig(
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
    pub fn exec_build_stdtx(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("tx")
            .and(warp::post())
            .and(warp::path::param::<StdTxType>())
            .and(auth())
            .and_then(handlers::stdtx_exec_build)
    }

    /// Build a MultiSig transaction
    pub fn exec_finalize_stdtx(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("tx")
            .and(warp::path("fn"))
            .and(warp::post())
            .and(warp::path::param::<StdTxType>())
            .and(warp::path::param::<String>())
            .and(auth())
            .and_then(handlers::stdtx_exec_finalize)
    }

    pub(crate) fn auth(
    ) -> impl Filter<Extract = ((u64, TXPWrapper),), Error = warp::Rejection> + Clone {
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
}

mod auth {
    use crate::error::{self, Error};
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
    use serde::{Deserialize, Serialize};
    use warp::{
        http::header::{HeaderMap, HeaderValue, AUTHORIZATION},
        reject, Rejection,
    };

    use hugin::{
        client::connect, OneShotMintPayload, TXPWrapper, TransactionPattern,
        WalletTransactionPattern,
    };
    use hugin::{Signature, VerifyUser};

    const BEARER: &str = "Bearer ";

    #[derive(Debug, Deserialize, Serialize)]
    struct ApiClaims {
        sub: String,
        exp: usize,
    }

    pub(crate) async fn authorize(
        headers: HeaderMap<HeaderValue>,
        body: bytes::Bytes,
    ) -> Result<(u64, TXPWrapper), Rejection> {
        let publ = std::env::var("JWT_PUB_KEY")
            .map_err(|_| Error::Custom("env jwt pub not existing".to_string()))?;
        let publ = publ.into_bytes();
        log::info!("checking login data ...");
        let b = Vec::<u8>::from(body);
        let txp_out = if let Ok(txp) =
            serde_json::from_str::<TransactionPattern>(std::str::from_utf8(&b).unwrap())
        {
            TXPWrapper::TransactionPattern(Box::new(txp))
        } else if let Ok(s) = serde_json::from_str::<Signature>(std::str::from_utf8(&b).unwrap()) {
            TXPWrapper::Signature(s)
        } else if let Ok(wal) =
            serde_json::from_str::<WalletTransactionPattern>(std::str::from_utf8(&b).unwrap())
        {
            TXPWrapper::TransactionPattern(Box::new(wal.into_txp()))
        } else {
            TXPWrapper::OneShotMinter(
                serde_json::from_str::<OneShotMintPayload>(std::str::from_utf8(&b).unwrap())
                    .unwrap(),
            )
        };
        log::debug!("\n\nBody: {b:?}\n\n");
        match jwt_from_header(&headers) {
            Ok(jwt) => {
                let decoded = decode::<ApiClaims>(
                    &jwt,
                    &DecodingKey::from_ec_pem(&publ).unwrap(),
                    &Validation::new(Algorithm::ES256),
                )
                .map_err(|_| reject::custom(Error::JWTTokenError))?;
                log::info!("lookup user data ...");
                let user_id = decoded.claims.sub.parse::<u64>().map_err(|_| {
                    reject::custom(Error::Custom("Could not parse customer id".to_string()))
                })?;
                // Deactivates User Identification, only API token validity checked
                //let mut client = connect(std::env::var("ODIN_URL").unwrap()).await.unwrap();
                //let cmd = VerifyUser::new(user_id, jwt);
                //log::info!("try to verify user ...");
                //match client.build_cmd::<VerifyUser>(cmd).await {
                //    Ok(_) => {}
                //    Err(_) => {
                //        return Err(reject::custom(Error::JWTTokenError));
                //    }
                //};
                log::debug!("Authentication successful: User_id: {user_id:?}; txp: {txp_out:?}");
                Ok((user_id, txp_out))
            }

            Err(e) => {
                println!("Authentication not successful");
                Err(reject::custom(e))
            }
        }
    }

    fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String, error::Error> {
        let header = match headers.get(AUTHORIZATION) {
            Some(v) => v,
            None => return Err(Error::NoAuthHeaderError),
        };
        let auth_header = match std::str::from_utf8(header.as_bytes()) {
            Ok(v) => v,
            Err(_) => return Err(Error::NoAuthHeaderError),
        };
        if !auth_header.starts_with(BEARER) {
            return Err(Error::InvalidAuthHeaderError);
        }
        Ok(auth_header.trim_start_matches(BEARER).to_owned())
    }
}

///Handlers
mod handlers {
    use hugin::client::{connect, Client};
    use hugin::datamodel::models::{
        ContractAction, ContractType, MarketplaceActions, MultiSigType, ReturnError, StdTxType,
        TxHash, UnsignedTransaction,
    };
    use hugin::{
        BuildContract, BuildMultiSig, BuildStdTx, FinalizeContract, FinalizeMultiSig,
        FinalizeStdTx, TXPWrapper,
    };
    use std::env;
    use std::{convert::Infallible, str::FromStr};

    async fn connect_odin() -> Client {
        connect(env::var("ODIN_URL").unwrap()).await.unwrap()
    }

    pub async fn contracts_list() -> Result<impl warp::Reply, Infallible> {
        let mut ret = Vec::<String>::new();

        for t in ContractType::CONTRTYPES.iter() {
            ret.push(t.to_string())
        }

        for t in MultiSigType::MULTISIGTYPES.iter() {
            ret.push(t.to_string())
        }

        Ok(warp::reply::json(&ret))
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
}