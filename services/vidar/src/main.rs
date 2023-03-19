/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
#![allow(opaque_hidden_inferred_bound)]
extern crate pretty_env_logger;
mod error;

use std::env;
use std::str;
use warp::Filter;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "4101";

#[tokio::main]
async fn main() {
    let host: String = env::var("POD_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string()); //cli.host.as_deref().unwrap_or(DEFAULT_HOST);
    let port = env::var("POD_PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string()); //cli.port.as_deref().unwrap_or(DEFAULT_PORT);

    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "vidar=info");
    }
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
    let routes = api.with(cors).with(warp::log("vidar"));
    let server = host.to_string() + ":" + &port;
    let socket: std::net::SocketAddr = server.parse().expect("Unable to parse socket address");

    warp::serve(routes).run(socket).await;
}

mod auth {
    use crate::error::{self, VError};
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
    use serde::{Deserialize, Serialize};
    use warp::{
        http::header::{HeaderMap, HeaderValue, AUTHORIZATION},
        reject, Rejection,
    };

    use hugin::client::connect;
    use hugin::VerifyUser;

    const BEARER: &str = "Bearer ";

    #[derive(Debug, Deserialize, Serialize)]
    struct ApiClaims {
        sub: String,
        exp: usize,
    }

    pub(crate) async fn authorize(headers: HeaderMap<HeaderValue>) -> Result<u64, Rejection> {
        let publ = std::env::var("JWT_PUB_KEY")
            .map_err(|_| VError::Custom("env jwt pub not existing".to_string()))?;
        let publ = publ.into_bytes();
        log::info!("checking login data ...");
        match jwt_from_header(&headers) {
            Ok(jwt) => {
                let decoded = decode::<ApiClaims>(
                    &jwt,
                    &DecodingKey::from_ec_pem(&publ).unwrap(),
                    &Validation::new(Algorithm::ES256),
                )
                .map_err(|_| reject::custom(VError::JWTTokenError))?;
                log::info!("lookup user data ...");
                let user_id = decoded.claims.sub.parse::<u64>().map_err(|_| {
                    reject::custom(VError::Custom("Could not parse customer id".to_string()))
                })?;
                let mut client = connect(std::env::var("ODIN_URL").unwrap()).await.unwrap();
                let cmd = VerifyUser::new(user_id, jwt);
                log::info!("try to verify user ...");
                match client.build_cmd::<VerifyUser>(cmd).await {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(reject::custom(VError::JWTTokenError));
                    }
                };
                Ok(user_id)
            }
            Err(e) => Err(reject::custom(e)),
        }
    }

    fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String, error::VError> {
        let header = match headers.get(AUTHORIZATION) {
            Some(v) => v,
            None => return Err(VError::NoAuthHeaderError),
        };
        let auth_header = match std::str::from_utf8(header.as_bytes()) {
            Ok(v) => v,
            Err(_) => return Err(VError::NoAuthHeaderError),
        };
        if !auth_header.starts_with(BEARER) {
            return Err(VError::InvalidAuthHeaderError);
        }
        Ok(auth_header.trim_start_matches(BEARER).to_owned())
    }
}

///Filters
mod filters {
    use super::handlers;
    use warp::Filter;

    pub fn endpoints() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_all_rewards_for_stake_addr()
            .or(get_rewards_for_stake_addr())
            .or(get_claim_history_for_stake_addr_contr())
            .or(get_claim_history_for_stake_addr())
            .or(get_total_rewards())
            .or(get_token_info())
            .or(get_user_tokens())
            .or(get_avail_mintrewards())
            .or(get_cl_rewards_for_stake_addr())
            .or(post_assethandles())
            .or(get_avail_mintrewards_user())
            .or(resp_option())
        // .or(warp::get().and(warp::any().map(warp::reply)))
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
    /// Get all available rewards for a stake address
    pub fn get_all_rewards_for_stake_addr(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("rwd")
            .and(warp::path("all"))
            .and(warp::get())
            .and(auth())
            .and(warp::path::param::<String>())
            .and_then(handlers::handle_all_rewards_for_stake_addr)
    }

    /// Get all available rewards for a client a stake address
    pub fn get_cl_rewards_for_stake_addr(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("rwd")
            .and(warp::path("cl"))
            .and(warp::get())
            .and(auth())
            .and(warp::path::param::<String>())
            .and_then(handlers::handle_rewards_for_client_stake_addr)
    }

    /// Get rewards for a stake address for a specific contract
    pub fn get_rewards_for_stake_addr(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("rwd")
            .and(warp::path("one"))
            .and(warp::get())
            .and(auth())
            .and(warp::path::param::<u64>()) // contract-id
            .and(warp::path::param::<String>()) //Stake_addr
            .and_then(handlers::handle_rewards_for_stake_addr)
    }

    /// Get claim history for a stake address for a specific contract
    pub fn get_claim_history_for_stake_addr_contr(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("rwd")
            .and(warp::path("history"))
            .and(warp::get())
            .and(auth())
            .and(warp::path::param::<u64>()) // contract-id
            .and(warp::path::param::<String>()) //Stake_addr
            .and_then(handlers::handle_claim_history_for_stake_addr_contr)
    }

    /// Get claim history for a stake address for a specific contract
    pub fn get_claim_history_for_stake_addr(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("rwd")
            .and(warp::path("history"))
            .and(warp::get())
            .and(auth())
            .and(warp::path::param::<String>()) //Stake_addr
            .and_then(handlers::handle_claim_history_for_stake_addr)
    }

    pub fn get_token_info(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("token")
            .and(warp::path("info"))
            .and(warp::get())
            .and(auth())
            .and(warp::path::param::<String>()) //fingerprint
            .and_then(handlers::handle_token_info)
    }

    pub fn get_user_tokens(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("tokens")
            .and(warp::get())
            .and(auth())
            .and_then(handlers::handle_tokens)
    }

    pub fn get_total_rewards(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::get()
            .and(warp::path("tokens"))
            .and(warp::path("rwd"))
            .and(auth())
            .and_then(handlers::handle_total_rewards)
    }

    pub fn get_avail_mintrewards(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("mird")
            .and(warp::path("all"))
            .and(warp::get())
            .and(auth())
            .and(warp::path::param::<String>())
            .and_then(handlers::handle_all_mint_rewards_for_stake_addr)
    }

    pub fn get_avail_mintrewards_user(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("mird")
            .and(warp::path("cl"))
            .and(warp::get())
            .and(auth())
            .and(warp::path::param::<String>())
            .and_then(handlers::handle_cl_mint_rewards_for_stake_addr)
    }

    pub fn post_assethandles(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("wallet")
            .and(warp::path("assets"))
            .and(warp::path("addresses"))
            .and(warp::post())
            .and(auth())
            .and(warp::body::content_length_limit(10000 * 1024).and(warp::body::json()))
            .and_then(handlers::handle_asset_for_addresses)
    }

    fn auth() -> impl Filter<Extract = (u64,), Error = warp::Rejection> + Clone {
        use super::auth::authorize;
        use warp::{
            filters::header::headers_cloned,
            http::header::{HeaderMap, HeaderValue},
        };
        headers_cloned()
            .map(move |headers: HeaderMap<HeaderValue>| (headers))
            .and_then(authorize)
    }
}

///Handlers
mod handlers {
    use cardano_serialization_lib::{
        address::Address, crypto::ScriptHash, utils::from_bignum, AssetName,
    };
    use gungnir::models::{MintProject, MintReward};
    use hugin::{
        datamodel::{ClaimedHandle, RewardHandle},
        MintProjectHandle, MintRewardHandle,
    };
    use murin::make_fingerprint;
    use std::{convert::Infallible, str::from_utf8};

    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
    pub struct ReturnError {
        pub msg: String,
    }

    impl ReturnError {
        pub fn new(str: &str) -> ReturnError {
            ReturnError {
                msg: str.to_string(),
            }
        }
    }

    pub async fn handle_all_rewards_for_stake_addr(
        _: u64,
        stake_addr: String,
    ) -> Result<impl warp::Reply, Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => {
                return make_error(e);
            }
        };
        let mut gconn =
            gungnir::establish_connection().expect("Error: Could not connect to Reward Database");
        let rewards = gungnir::Rewards::get_rewards_stake_addr(&mut gconn, bech32addr);
        println!("Rewards: {:?}", rewards);
        let response = match rewards {
            Ok(rwds) => {
                let mut ret = Vec::<RewardHandle>::new();
                for rwd in rwds {
                    match gungnir::TokenWhitelist::get_token_info_ft(&mut gconn, &rwd.fingerprint) {
                        Ok(ti) => ret.push(RewardHandle::new(&ti, &rwd)),
                        Err(_) => {
                            log::info!(
                                "Error: could not find token info for {:?}",
                                rwd.fingerprint
                            );
                        }
                    }
                }
                warp::reply::json(&ret)
            }
            Err(otherwise) => {
                log::info!("{:?}", otherwise);
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };

        Ok(warp::reply::with_status(
            response,
            warp::http::StatusCode::OK,
        ))
    }

    /// execute build multisig for <multisig_type> for customer <customer_id> with <payload>
    pub async fn handle_rewards_for_stake_addr(
        customer_id: u64,
        contract_id: u64,
        stake_addr: String,
    ) -> Result<impl warp::Reply, Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => {
                return make_error(e);
            }
        };
        let mut gconn =
            gungnir::establish_connection().expect("Error: Could not connect to Reward Database");
        let rewards = gungnir::Rewards::get_rewards(
            &mut gconn,
            bech32addr,
            contract_id as i64,
            customer_id as i64,
        );

        let response = match rewards {
            Ok(rwds) => {
                let mut ret = Vec::<RewardHandle>::new();
                for rwd in rwds {
                    match gungnir::TokenWhitelist::get_token_info_ft(&mut gconn, &rwd.fingerprint) {
                        Ok(ti) => ret.push(RewardHandle::new(&ti, &rwd)),
                        Err(_) => {
                            log::info!(
                                "Error: coudl not find token info for {:?}",
                                rwd.fingerprint
                            );
                        }
                    }
                }
                warp::reply::json(&ret)
            }
            Err(otherwise) => {
                log::info!("{:?}", otherwise);
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };

        Ok(warp::reply::with_status(
            response,
            warp::http::StatusCode::OK,
        ))
    }

    /// execute build multisig for <multisig_type> for customer <customer_id> with <payload>
    pub async fn handle_rewards_for_client_stake_addr(
        customer_id: u64,
        stake_addr: String,
    ) -> Result<impl warp::Reply, Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => {
                return make_error(e);
            }
        };
        let mut gconn =
            gungnir::establish_connection().expect("Error: Could not connect to Reward Database");
        let rewards =
            gungnir::Rewards::get_client_rewards(&mut gconn, bech32addr, customer_id as i64);

        let response = match rewards {
            Ok(rwds) => {
                let mut ret = Vec::<RewardHandle>::new();
                for rwd in rwds {
                    match gungnir::TokenWhitelist::get_token_info_ft(&mut gconn, &rwd.fingerprint) {
                        Ok(ti) => ret.push(RewardHandle::new(&ti, &rwd)),
                        Err(_) => {
                            log::info!(
                                "Error: coudl not find token info for {:?}",
                                rwd.fingerprint
                            );
                        }
                    }
                }
                warp::reply::json(&ret)
            }
            Err(otherwise) => {
                log::info!("{:?}", otherwise);
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };

        Ok(warp::reply::with_status(
            response,
            warp::http::StatusCode::OK,
        ))
    }

    /// handle_claim_history_for_stake_addr and specific contract
    pub async fn handle_claim_history_for_stake_addr_contr(
        customer_id: u64,
        contract_id: u64,
        stake_addr: String,
    ) -> Result<impl warp::Reply, Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => {
                return make_error(e);
            }
        };
        let mut gconn =
            gungnir::establish_connection().expect("Error: Could not connect to Reward Database");
        let claims = gungnir::Claimed::get_claims(
            &mut gconn,
            &bech32addr,
            contract_id as i64,
            customer_id as i64,
        );
        let response = match claims {
            Ok(clms) => {
                let mut ret = Vec::<ClaimedHandle>::new();
                for clm in clms {
                    match gungnir::TokenWhitelist::get_token_info_ft(&mut gconn, &clm.fingerprint) {
                        Ok(cl) => ret.push(ClaimedHandle::new(
                            clm.stake_addr,
                            clm.payment_addr,
                            cl.policy,
                            cl.tokenname.unwrap(),
                            cl.fingerprint.unwrap(),
                            clm.amount,
                            clm.contract_id,
                            clm.user_id,
                            clm.txhash,
                            clm.invalid,
                            clm.invalid_descr,
                            clm.timestamp,
                            clm.updated_at,
                        )),
                        Err(e) => {
                            log::info!(
                                "Error: coudl not find token info for {:?}, {:?}",
                                clm.fingerprint,
                                e
                            );
                            return Ok(warp::reply::with_status(
                                warp::reply::json(&ReturnError::new(&e.to_string())),
                                warp::http::StatusCode::NOT_FOUND,
                            ));
                        }
                    }
                }
                warp::reply::json(&ret)
            }
            Err(otherwise) => {
                log::info!("{:?}", otherwise);
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };

        Ok(warp::reply::with_status(
            response,
            warp::http::StatusCode::OK,
        ))
    }

    /// handle_claim_history_for_stake_addr and specific contract
    pub async fn handle_claim_history_for_stake_addr(
        _: u64,
        stake_addr: String,
    ) -> Result<impl warp::Reply, Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => {
                return make_error(e);
            }
        };
        let mut gconn =
            gungnir::establish_connection().expect("Error: Could not connect to Reward Database");
        let claims = gungnir::Claimed::get_all_claims(&mut gconn, &bech32addr);

        let response = match claims {
            Ok(clms) => {
                let mut ret = Vec::<ClaimedHandle>::new();
                for clm in clms {
                    match gungnir::TokenWhitelist::get_token_info_ft(&mut gconn, &clm.fingerprint) {
                        Ok(cl) => ret.push(ClaimedHandle::new(
                            clm.stake_addr,
                            clm.payment_addr,
                            cl.policy,
                            cl.tokenname.unwrap(),
                            cl.fingerprint.unwrap(),
                            clm.amount,
                            clm.contract_id,
                            clm.user_id,
                            clm.txhash,
                            clm.invalid,
                            clm.invalid_descr,
                            clm.timestamp,
                            clm.updated_at,
                        )),
                        Err(e) => {
                            log::info!(
                                "Error: coudl not find token info for {:?}, {:?}",
                                clm.fingerprint,
                                e
                            );
                            return Ok(warp::reply::with_status(
                                warp::reply::json(&ReturnError::new(&e.to_string())),
                                warp::http::StatusCode::NOT_FOUND,
                            ));
                        }
                    }
                }
                warp::reply::json(&ret)
            }
            Err(otherwise) => {
                log::info!("{:?}", otherwise);
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };

        Ok(warp::reply::with_status(
            response,
            warp::http::StatusCode::OK,
        ))
    }

    pub async fn handle_token_info(
        _: u64,
        fingerprint: String,
    ) -> Result<impl warp::Reply, Infallible> {
        let response = match mimir::get_mint_metadata(&fingerprint) {
            Ok(t) => t,
            Err(e) => {
                log::info!(
                    "Error: could not find token info for {:?}, {:?}",
                    fingerprint,
                    e
                );
                return Ok(warp::reply::with_status(
                    warp::reply::json(&ReturnError::new(&e.to_string())),
                    warp::http::StatusCode::NOT_FOUND,
                ));
            }
        };

        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!(response)),
            warp::http::StatusCode::OK,
        ))
    }

    pub async fn handle_tokens(user_id: u64) -> Result<impl warp::Reply, Infallible> {
        let response = match gungnir::TokenWhitelist::get_user_tokens(&user_id) {
            Ok(t) => t,
            Err(e) => {
                log::info!("Error: could not find any tokens");
                return Ok(warp::reply::with_status(
                    warp::reply::json(&ReturnError::new(&e.to_string())),
                    warp::http::StatusCode::NOT_FOUND,
                ));
            }
        };

        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!(response)),
            warp::http::StatusCode::OK,
        ))
    }

    pub async fn handle_total_rewards(user_id: u64) -> Result<impl warp::Reply, Infallible> {
        match gungnir::Rewards::get_total_rewards_token(user_id as i64) {
            Ok(t) => Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!(t)),
                warp::http::StatusCode::OK,
            )),
            Err(e) => {
                log::info!("Error: could not find any tokens");
                Ok(warp::reply::with_status(
                    warp::reply::json(&ReturnError::new(&e.to_string())),
                    warp::http::StatusCode::NOT_FOUND,
                ))
            }
        }
    }

    pub fn make_error(e: String) -> Result<warp::reply::WithStatus<warp::reply::Json>, Infallible> {
        Ok(warp::reply::with_status(
            warp::reply::json(&ReturnError::new(&e)),
            warp::http::StatusCode::NOT_ACCEPTABLE,
        ))
    }

    pub fn get_bech32_from_bytes(stake_addr_bytes: String) -> Result<String, String> {
        let err = "; Error: Could not construct bech32 Address".to_string();
        match hex::decode(stake_addr_bytes) {
            Ok(h) => match Address::from_bytes(h) {
                Ok(a) => match a.to_bech32(None) {
                    Ok(s) => Ok(s),
                    Err(e) => Err(e.to_string() + &err),
                },
                Err(e) => Err(e.to_string() + &err),
            },
            Err(e) => Err(e.to_string() + &err),
        }
    }

    pub async fn handle_all_mint_rewards_for_stake_addr(
        _: u64,
        stake_addr: String,
    ) -> Result<impl warp::Reply, Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => {
                return make_error(e);
            }
        };

        let payaddr = match mimir::select_addr_of_first_transaction(&bech32addr) {
            Ok(a) => a,
            Err(e) => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&e.to_string()),
                    warp::http::StatusCode::NO_CONTENT,
                ))
            }
        };

        let rewards = MintReward::get_avail_mintrewards_by_addr(&payaddr);
        println!("Rewards: {:?}", rewards);
        match rewards {
            Ok(rwds) => {
                let mut ret = Vec::<MintRewardHandle>::new();
                for rwd in rwds {
                    match MintProject::get_mintproject_by_id_active(rwd.project_id) {
                        Ok(p) => ret.push(MintRewardHandle {
                            id: rwd.id,
                            addr: rwd.pay_addr,
                            project: MintProjectHandle {
                                project_name: p.project_name,
                                collection_name: p.collection_name,
                                author: p.author,
                                image: None,
                            },
                        }),
                        Err(e) => {
                            log::info!("Error: could not find active mint project");
                            return Ok(warp::reply::with_status(
                                warp::reply::json(&e.to_string()),
                                warp::http::StatusCode::NO_CONTENT,
                            ));
                        }
                    }
                }

                Ok(warp::reply::with_status(
                    warp::reply::json(&ret),
                    warp::http::StatusCode::OK,
                ))
            }
            Err(otherwise) => {
                log::info!("{:?}", otherwise);

                Ok(warp::reply::with_status(
                    warp::reply::json(&ReturnError::new(&otherwise.to_string())),
                    warp::http::StatusCode::NO_CONTENT,
                ))
            }
        }
    }

    pub async fn handle_cl_mint_rewards_for_stake_addr(
        user_id: u64,
        stake_addr: String,
    ) -> Result<impl warp::Reply, Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => {
                return make_error(e);
            }
        };

        let payaddr = match mimir::select_addr_of_first_transaction(&bech32addr) {
            Ok(a) => a,
            Err(e) => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&e.to_string()),
                    warp::http::StatusCode::NO_CONTENT,
                ))
            }
        };

        let rewards = MintReward::get_avail_mintrewards_cl_by_addr(user_id as i64, &payaddr);
        println!("Rewards: {rewards:?}");
        match rewards {
            Ok(rwds) => {
                let mut ret = Vec::<MintRewardHandle>::new();
                for rwd in rwds {
                    match MintProject::get_mintproject_by_id_active(rwd.project_id) {
                        Ok(p) => ret.push(MintRewardHandle {
                            id: rwd.id,
                            addr: rwd.pay_addr,
                            project: MintProjectHandle {
                                project_name: p.project_name,
                                collection_name: p.collection_name,
                                author: p.author,
                                image: None,
                            },
                        }),
                        Err(e) => {
                            log::info!("Error: could not find active mint project");
                            return Ok(warp::reply::with_status(
                                warp::reply::json(&e.to_string()),
                                warp::http::StatusCode::NO_CONTENT,
                            ));
                        }
                    }
                }

                Ok(warp::reply::with_status(
                    warp::reply::json(&ret),
                    warp::http::StatusCode::OK,
                ))
            }
            Err(otherwise) => {
                log::info!("{:?}", otherwise);

                Ok(warp::reply::with_status(
                    warp::reply::json(&ReturnError::new(&otherwise.to_string())),
                    warp::http::StatusCode::NO_CONTENT,
                ))
            }
        }
    }

    pub async fn handle_asset_for_addresses(
        _: u64,
        addresses: Vec<String>,
    ) -> Result<impl warp::Reply, Infallible> {
        /*
        let bech32addr = match get_bech32_from_bytes(&stake_addr) {
            Ok(s) => s,
            Err(e) => {
                return make_error(e);
            }
        };

        let ident_address = match mimir::select_addr_of_first_transaction(&bech32addr) {
            Ok(a) => a,
            Err(e) => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&e.to_string()),
                    warp::http::StatusCode::NO_CONTENT,
                ))
            }
        };
         */

        let mut utxos = murin::TransactionUnspentOutputs::new();

        for a in &addresses {
            let us = mimir::get_address_utxos(a).unwrap();
            utxos.merge(us);
        }

        let mut handles = Vec::<hugin::AssetHandle>::new();
        for u in utxos {
            let v = u.output().amount();
            let ada = v.coin();
            handles.push(hugin::AssetHandle {
                fingerprint: None,
                policy: None,
                tokenname: None,
                amount: from_bignum(&ada),
                metadata: None,
            });
            if let Some(multis) = v.multiasset() {
                let policies = multis.keys();
                for p in 0..policies.len() {
                    let policy = policies.get(p);
                    if let Some(assets) = multis.get(&policy) {
                        let k = assets.keys();
                        for a in 0..k.len() {
                            let asset = k.get(a);
                            let amt = assets.get(&asset).unwrap();
                            let fingerprint =
                                make_fingerprint(&policy.to_hex(), &hex::encode(asset.name()))
                                    .unwrap();
                            handles.push(hugin::AssetHandle {
                                fingerprint: Some(fingerprint),
                                policy: Some(policy.to_hex()),
                                tokenname: Some(from_utf8(&asset.name()).unwrap().to_owned()),
                                amount: from_bignum(&amt),
                                metadata: None,
                            })
                        }
                    }
                }
            }
        }

        let mut handles_summed = Vec::<hugin::AssetHandle>::new();

        for h in &handles {
            if handles_summed
                .iter()
                .filter(|n| h.same_asset(n))
                .collect::<Vec<&hugin::AssetHandle>>()
                .is_empty()
            {
                let sum = handles
                    .iter()
                    .fold(hugin::AssetHandle::new_empty(), |mut acc, f| {
                        if h.same_asset(f) {
                            acc.amount = acc.amount.checked_add(h.amount).unwrap();

                            if acc.metadata.is_none() && f.metadata.is_some() {
                                acc.metadata = h.metadata.clone()
                            }
                            if acc.fingerprint.is_none() && f.fingerprint.is_some() {
                                acc.fingerprint = h.fingerprint.clone()
                            }
                            if acc.policy.is_none() && f.policy.is_some() {
                                acc.policy = h.policy.clone()
                            }
                            if acc.tokenname.is_none() && f.tokenname.is_some() {
                                acc.tokenname = h.tokenname.clone()
                            }
                        }
                        acc
                    });
                handles_summed.push(sum)
            }
        }

        Ok(warp::reply::with_status(
            warp::reply::json(&handles_summed),
            warp::http::StatusCode::OK,
        ))
    }
}
