/*
######################################################################
# See LICENSE.md for full license information.                       #
# Software: Drasil Blockchain Application Framework                  #
# License: “Commons Clause” License Condition v1.0 & Apache 2.0      #
# Licensor: Torben Poguntke (torben@drasil.io)                       #
######################################################################
*/
extern crate pretty_env_logger;

use warp::Filter;
use std::env;
use std::str;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

const DEFAULT_HOST : &str = "127.0.0.1";
const DEFAULT_PORT : &str = "4101";

#[tokio::main]
async fn main() {

    let host : String =  env::var("POD_HOST").unwrap_or(DEFAULT_HOST.to_string()); //cli.host.as_deref().unwrap_or(DEFAULT_HOST);
    let port = env::var("POD_PORT").unwrap_or(DEFAULT_PORT.to_string()); //cli.port.as_deref().unwrap_or(DEFAULT_PORT);

    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "vidar=info");
    }
    pretty_env_logger::init();


    let cors2 = warp::cors().allow_any_origin().allow_methods(vec!["GET", "POST", "OPTIONS", "PUT"]).allow_credentials(true).allow_headers(vec![
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
        "Sec-Fetch-Site"]);


    let api = filters::endpoints();
    let routes = api.with(cors2).with(warp::log("vidar"));
    let server = host.to_string()+":"+&port;
    let socket : std::net::SocketAddr = server.parse().expect("Unable to parse socket address");

        warp::serve(routes).run(socket).await;
}


///Filters
mod filters {
    use super::handlers;
    use warp::{Filter}; 

    pub fn endpoints( ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_all_rewards_for_stake_addr()
        .or(get_rewards_for_stake_addr()) 
        .or(get_claim_history_for_stake_addr_contr())
        .or(get_claim_history_for_stake_addr())
        .or(resp_option())
        .or(warp::get().and(warp::any().map(warp::reply)))
    }

    pub fn resp_option( ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
       warp::options().and(warp::header("origin")).map(|origin : String| {
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
            .and(warp::path::param::<String>())
            .and(auth()) 
            .and_then(handlers::handle_all_rewards_for_stake_addr)          
    }

     /// Get rewards for a stake address for a specific contract
     pub fn get_rewards_for_stake_addr(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("rwd")
            .and(warp::path("one"))
            .and(warp::get())    
            .and(warp::path::param::<u64>())// customer id
            .and(warp::path::param::<u64>())// contract-id
            .and(warp::path::param::<String>())//Stake_addr
            .and(auth())
            .and_then(handlers::handle_rewards_for_stake_addr)
    }


    /// Get claim history for a stake address for a specific contract
    pub fn get_claim_history_for_stake_addr_contr(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("rwd")
            .and(warp::path("history"))
            .and(warp::get())    
            .and(warp::path::param::<u64>())// customer id
            .and(warp::path::param::<u64>())// contract-id
            .and(warp::path::param::<String>())//Stake_addr
            .and(auth())
            .and_then(handlers::handle_claim_history_for_stake_addr_contr)
    }

    /// Get claim history for a stake address for a specific contract
    pub fn get_claim_history_for_stake_addr(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("rwd")
            .and(warp::path("history"))
            .and(warp::get())    
            .and(warp::path::param::<String>())//Stake_addr
            .and(auth())
            .and_then(handlers::handle_claim_history_for_stake_addr)
    }
/*
    pub fn get_token_info(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("token")
            .and(warp::path("info"))
            .and(warp::get())    
            .and(warp::path::param::<String>())//Stake_addr
            .and(auth())
            .and_then(handlers::handle_token_info)
    }
*/
    fn auth() -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
        // TODO : implement authorization via JWT
        warp::header::exact("authorization", "***REMOVED***")
    }

}



///Handlers
mod handlers {
    use std::{convert::Infallible};
    use bigdecimal::{BigDecimal,FromPrimitive,ToPrimitive}; //ToPrimitive
    use cardano_serialization_lib::address::Address as Address;
    use hex;
    use chrono::{DateTime,Utc};

    #[derive(serde::Serialize, serde::Deserialize,Debug,Clone)]
    pub struct ReturnError {
        pub msg: String,
    }

    impl ReturnError {
        pub fn new(str : &String) -> ReturnError {
            ReturnError {
                msg : str.clone(),
            }
        }
    }

    #[derive(serde::Serialize, serde::Deserialize,Debug,Clone)]
    pub struct ClaimedResponse {
        pub stake_addr          : String,
        pub payment_addr        : String,
        pub policyid            : String,
        pub tokenname           : String,
        pub fingerprint         : String,
        pub amount              : BigDecimal,
        pub contract_id         : i64,
        pub user_id             : i64, 
        pub txhash              : String,
        pub invalid             : Option<bool>, 
        pub invalid_descr       : Option<String>, 
        pub timestamp           : DateTime<Utc>,
        pub updated_at          : DateTime<Utc>,
    }

    impl ClaimedResponse {
        pub fn new(
            stake_addr          : String,
            payment_addr        : String,
            policyid            : String,
            tokenname           : String,
            fingerprint         : String,
            amount              : BigDecimal,
            contract_id         : i64,
            user_id             : i64, 
            txhash              : String,
            invalid             : Option<bool>, 
            invalid_descr       : Option<String>, 
            timestamp           : DateTime<Utc>,
            updated_at          : DateTime<Utc>,
        ) -> ClaimedResponse {
            ClaimedResponse {
                stake_addr,
                payment_addr,
                policyid,
                tokenname,
                fingerprint,
                amount,
                contract_id,
                user_id,
                txhash,
                invalid,
                invalid_descr,
                timestamp,
                updated_at,
            }

        }
    }

    #[derive(serde::Serialize, serde::Deserialize,Debug,Clone)]
    pub struct RewardResponse{
        pub stake_addr          : String,
        pub fingerprint         : String,
        pub policy              : String,
        pub tokenname           : String,
        pub tot_earned          : BigDecimal,
        pub tot_claimed         : BigDecimal,
        pub last_calc_epoch     : i64,
    }

    impl RewardResponse {
        pub fn new(
            stake_addr          : String,
            fingerprint         : String,
            policy              : String,
            tokenname           : String,
            tot_earned          : BigDecimal,
            tot_claimed         : BigDecimal,
            last_calc_epoch     : i64,
        ) -> RewardResponse {
            RewardResponse {
                stake_addr,
                fingerprint,
                policy,
                tokenname,
                tot_earned,
                tot_claimed,
                last_calc_epoch,
            }

        }
    }

    pub async fn handle_all_rewards_for_stake_addr(stake_addr : String) -> Result<impl warp::Reply, Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => { return make_error(e); }
        };
        let gconn = gungnir::establish_connection().expect("Error: Could not connect to Reward Database");
        let rewards = gungnir::Rewards::get_rewards_stake_addr(
            &gconn,
            bech32addr
        );
        println!("Rewards: {:?}",rewards);
        let lovelace = BigDecimal::from_i32(1000000).unwrap();
        let response = match rewards {
            Ok(rwds) => {
                let mut ret = Vec::<RewardResponse>::new();
                for rwd in rwds {
                    match gungnir::TokenWhitelist::get_token_info_ft(
                        &gconn,
                        &rwd.fingerprint
                    ) {
                       
                        
                        Ok(ti) => {
                            ret.push(
                                RewardResponse::new(
                                    rwd.stake_addr, 
                                    ti.fingerprint.unwrap(), 
                                    ti.policy,
                                    ti.tokenname.unwrap(), 
                                    BigDecimal::from_u64((rwd.tot_earned/&lovelace).to_u64().unwrap()).unwrap(), //rwd.tot_earned/&lovelace,//
                                    rwd.tot_claimed,
                                    rwd.last_calc_epoch)
                            )
                        },
                        Err(_) => { 
                            log::info!("Error: coudl not find token info for {:?}",rwd.fingerprint);
                        }
                    }

                }
                warp::reply::json(&ret)      
            },
            Err(otherwise) => {
                log::info!("{:?}",otherwise);
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };
        
        Ok(warp::reply::with_status(
            response, 
             warp::http::StatusCode::OK)
         )

    }

    /// execute build multisig for <multisig_type> for customer <customer_id> with <payload>
    pub async fn handle_rewards_for_stake_addr(customer_id: u64, contract_id: u64, stake_addr: String ) -> Result<impl warp::Reply,Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => { return make_error(e); }
        };
        let gconn = gungnir::establish_connection().expect("Error: Could not connect to Reward Database");
        let rewards = gungnir::Rewards::get_rewards(
            &gconn,
            bech32addr,
            contract_id as i64,
            customer_id as i64,
        );

        let response = match rewards {
            Ok(rwds) => {
                let mut ret = Vec::<RewardResponse>::new();
                let lovelace = BigDecimal::from_i32(1000000).unwrap();
                for rwd in rwds {
                    match gungnir::TokenWhitelist::get_token_info_ft(
                        &gconn,
                        &rwd.fingerprint
                    ) {
                        Ok(ti) => {
                            ret.push(
                                RewardResponse::new(
                                    rwd.stake_addr, 
                                    ti.fingerprint.unwrap(), 
                                    ti.policy,
                                    ti.tokenname.unwrap(), 
                                    BigDecimal::from_u64((rwd.tot_earned/&lovelace).to_u64().unwrap()).unwrap(), //rwd.tot_earned/&lovelace,//
                                    rwd.tot_claimed,
                                    rwd.last_calc_epoch)
                            )
                        },
                        Err(_) => { 
                            log::info!("Error: coudl not find token info for {:?}",rwd.fingerprint);
                        }
                    }

                }
                warp::reply::json(&ret)      
            },
            Err(otherwise) => {
                log::info!("{:?}",otherwise);
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };
        
        Ok(warp::reply::with_status(
            response, 
             warp::http::StatusCode::OK)
         )
    }  

    
    /// handle_claim_history_for_stake_addr and specific contract
    pub async fn handle_claim_history_for_stake_addr_contr(customer_id: u64, contract_id: u64, stake_addr: String ) -> Result<impl warp::Reply,Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => { return make_error(e); }
        };
        let gconn = gungnir::establish_connection().expect("Error: Could not connect to Reward Database");
        let claims = gungnir::Claimed::get_claims(
            &gconn,
            &bech32addr,
            contract_id as i64,
            customer_id as i64,
        );
        let response = match claims {
            Ok(clms) => {
                let mut ret = Vec::<ClaimedResponse>::new();
                for clm in clms {
                    match gungnir::TokenWhitelist::get_token_info_ft(
                        &gconn,
                        &clm.fingerprint
                    ) {
                        Ok(cl) => {
                            ret.push(
                                ClaimedResponse::new(
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
                            ))
                        },
                        Err(e) => { 
                            log::info!("Error: coudl not find token info for {:?}, {:?}",clm.fingerprint,e);
                            return Ok(warp::reply::with_status(warp::reply::json(&ReturnError::new(&e.to_string())),warp::http::StatusCode::NOT_FOUND));
                        }
                    }
                }
                warp::reply::json(&ret)      
            },
            Err(otherwise) => {
                log::info!("{:?}",otherwise);
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };
        
        Ok(warp::reply::with_status(
            response, 
             warp::http::StatusCode::OK)
         )
    }
    
    /// handle_claim_history_for_stake_addr and specific contract
    pub async fn handle_claim_history_for_stake_addr(stake_addr: String ) -> Result<impl warp::Reply,Infallible> {
        let bech32addr = match get_bech32_from_bytes(stake_addr) {
            Ok(s) => s,
            Err(e) => { return make_error(e); }
        };
        let gconn = gungnir::establish_connection().expect("Error: Could not connect to Reward Database");
        let claims = gungnir::Claimed::get_all_claims(
            &gconn,
            &bech32addr,
        );

        let response = match claims {
            Ok(clms) => {
                let mut ret = Vec::<ClaimedResponse>::new();
                for clm in clms {
                    match gungnir::TokenWhitelist::get_token_info_ft(
                        &gconn,
                        &clm.fingerprint
                    ) {
                        Ok(cl) => {
                            ret.push(
                                ClaimedResponse::new(
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
                            ))
                        },
                        Err(e) => { 
                            log::info!("Error: coudl not find token info for {:?}, {:?}",clm.fingerprint,e);
                            return Ok(warp::reply::with_status(warp::reply::json(&ReturnError::new(&e.to_string())),warp::http::StatusCode::NOT_FOUND));
                        }
                    }
                }
                warp::reply::json(&ret)      
            },
            Err(otherwise) => {
                log::info!("{:?}",otherwise);
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };
        
        Ok(warp::reply::with_status(
            response, 
             warp::http::StatusCode::OK)
         )
    }


    pub fn make_error(e: String) -> Result<warp::reply::WithStatus<warp::reply::Json>,Infallible> {
        Ok(warp::reply::with_status(warp::reply::json(&ReturnError::new(&e.to_string())),warp::http::StatusCode::NOT_ACCEPTABLE))
    }
    
    pub fn get_bech32_from_bytes(stake_addr_bytes: String) -> Result<String,String> {
        let err = "; Error: Could not construct bech32 Address".to_string();
        match hex::decode(stake_addr_bytes) {
            Ok(h) => {
                match Address::from_bytes(h) {
                    Ok(a) => {
                       match a.to_bech32(None) {
                            Ok(s) => Ok(s),
                            Err(e) => {
                                return Err(e.to_string()+&err)
                            }
                       }
                    },
                    Err(e) => {
                        return Err(e.to_string()+&err);
                    }
                }
            },
            Err(e) => {
                return Err(e.to_string()+&err);
            }
        }
    }
}
