/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
extern crate pretty_env_logger;

mod error;
mod clientapi;
use warp::Filter;
use std::env;
use std::str;


const DEFAULT_HOST : &str = "127.0.0.1";
const DEFAULT_PORT : &str = "4000";
//const MAX_CONNECTIONS: usize = 1000;

#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }

    let host : String =  env::var("POD_HOST").unwrap_or(DEFAULT_HOST.to_string()); //cli.host.as_deref().unwrap_or(DEFAULT_HOST);
    let port = env::var("POD_PORT").unwrap_or(DEFAULT_PORT.to_string()); //cli.port.as_deref().unwrap_or(DEFAULT_PORT);

    pretty_env_logger::init();
  
    let cors = warp::cors().allow_any_origin().allow_methods(vec!["GET", "POST", "OPTIONS", "PUT"]).allow_credentials(true).allow_headers(vec![
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
    // view access logs by setting RUST_LOG=hepha
    let routes = api.with(cors).with(warp::log("heimdallr"));
    // Start Server
    let server = host.clone()+":"+&port;
    let socket : std::net::SocketAddr = server.parse().expect("Unable to parse socket address");

    //dotenv::dotenv().ok();
    //let cert_path = env::var("CERT_PATH").unwrap();
    //let key_path = env::var("KEY_PATH").unwrap();

    warp::serve(routes).run(socket).await; //

    //if host == "127.0.0.1".to_string() && false {    
        //warp::serve(routes).run(socket).await; //
    //} else {
        //warp::serve(routes).tls().cert_path(Path::new(&cert_path)).key_path(Path::new(&key_path)).run(socket).await; //
    //}
    
}


///Filters
mod filters {
    use super::handlers;
    use hugin::datamodel::hephadata::{TransactionPattern,ContractType,MultiSigType,Signature,StdTxType};
    use warp::{Filter};
    use crate::clientapi::filter::{self, api_endpoints};
   

    pub fn endpoints( ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
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

    /// GET contracts 
    pub fn list_contracts(

    ) ->  impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {        
        warp::path!("lcn")
       //     .and(auth())
            .and(warp::get())
            .and_then(handlers::contracts_list)
    }

    /// Build a Smart Contract transaction
    pub fn exec_build_contract (
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("cn")
            .and(warp::post())
            .and(auth())
            .and(warp::path::param::<ContractType>())
            .and(warp::path::param::<String>())
            .and(json_body_build())
            .and_then(handlers::contract_exec_build)
    }

     /// Finalize a Contract transaction
     pub fn exec_finalize_contract (
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("cn")
            .and(warp::path("fn"))
            .and(warp::post())    
            .and(auth())
            .and(warp::path::param::<ContractType>())
            .and(warp::path::param::<String>())
            .and(json_body_finalize())
            .and_then(handlers::contract_exec_finalize)
    }

    /// Build a MultiSig transaction
    pub fn exec_build_multisig (
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("ms")
            .and(warp::post())    
            .and(auth())
            .and(warp::path::param::<MultiSigType>()) 
            .and(json_body_build())
            .and_then(handlers::multisig_exec_build)
    }


    /// Build a MultiSig transaction
    pub fn exec_finalize_multisig (
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("ms")
            .and(warp::path("fn"))
            .and(warp::post())    
            .and(auth())
            .and(warp::path::param::<MultiSigType>())
            .and(warp::path::param::<String>())
            .and(json_body_finalize())
            .and_then(handlers::multisig_exec_finalize)
    }

    /// Build a MultiSig transaction
    pub fn exec_build_stdtx (
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("tx")
            .and(warp::post())    
            .and(auth())
            .and(warp::path::param::<StdTxType>()) 
            .and(json_body_build())
            .and_then(handlers::stdtx_exec_build)
    }


    /// Build a MultiSig transaction
    pub fn exec_finalize_stdtx (
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("tx")
            .and(warp::path("fn"))
            .and(warp::post())    
            .and(auth())
            .and(warp::path::param::<StdTxType>())
            .and(warp::path::param::<String>())
            .and(json_body_finalize())
            .and_then(handlers::stdtx_exec_finalize)
    }

    fn json_body_build() -> impl Filter<Extract = (TransactionPattern,), Error = warp::Rejection> + Clone {
        warp::body::content_length_limit(100 * 1024).and(warp::body::json())
    }

    fn json_body_finalize() -> impl Filter<Extract = (Signature,), Error = warp::Rejection> + Clone {
        warp::body::content_length_limit(10 * 1024).and(warp::body::json())
    }


    fn auth() -> impl Filter<Extract = (u64,), Error = warp::Rejection> + Clone {
        use warp::{
            filters::header::headers_cloned,
            http::header::{HeaderMap, HeaderValue}
        };
        use super::auth::authorize;
        headers_cloned()
            .map(move |headers: HeaderMap<HeaderValue>| (headers))
            .and_then(authorize)
         
    }

}

mod auth {
    use crate::error::{self, Error};
    use warp::{
        http::header::{HeaderMap, HeaderValue, AUTHORIZATION},
        reject, Rejection,
    };
    use serde::{Deserialize,Serialize};
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

    use hugin::{VerifyUser};
    use hugin::client::{connect};

    const BEARER: &str = "Bearer ";

    #[derive(Debug, Deserialize, Serialize)]
    struct ApiClaims {
        sub: String,
        exp: usize,
    }

    pub(crate) async fn authorize(headers: HeaderMap<HeaderValue>) -> Result<u64,Rejection> {
        let publ = std::env::var("JWT_PUB_KEY").map_err(|_| Error::Custom("env jwt pub not existing".to_string()))?;
        let publ = publ.into_bytes();
        log::info!("checking login data ...");
        match jwt_from_header(&headers) {
            Ok(jwt) => {
                let decoded = decode::<ApiClaims>(
                    &jwt,
                    &DecodingKey::from_ec_pem(&publ).unwrap(),
                    &Validation::new(Algorithm::ES256),
                )
                .map_err(|_| reject::custom(Error::JWTTokenError))?;
                log::info!("lookup user data ...");
                let user_id = decoded.claims.sub.parse::<u64>().map_err(|_| reject::custom(Error::Custom("Could not parse customer id".to_string())))?;
                let mut client = connect(std::env::var("ODIN_URL").unwrap()).await.unwrap();
                let cmd = VerifyUser::new(user_id, jwt);      
                log::info!("try to verify user ...");
                match client.build_cmd::<VerifyUser>(cmd).await {
                    Ok(_) => {},
                    Err(_) => {
                        return Err(reject::custom(Error::JWTTokenError));
                    }
                };
                Ok(user_id)
            }
            Err(e) => return Err(reject::custom(e)),
        }
    }
    
    fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String,error::Error> {
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
    use hugin::{BuildContract, BuildMultiSig,FinalizeContract,FinalizeMultiSig, BuildStdTx, FinalizeStdTx};
    use hugin::datamodel::hephadata::{ 
        TransactionPattern,ContractType,MultiSigType,StdTxType,Signature,MarketplaceActions,ContractAction,UnsignedTransaction, TxHash, ReturnError
    };
    use hugin::client::{Client, connect};
    use std::{convert::Infallible, str::FromStr};
    use std::env;
    
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

    pub async fn contract_exec_build(customer_id : u64, contract : ContractType, action : String, payload : TransactionPattern) -> Result<impl warp::Reply, Infallible> {
        let badreq = warp::reply::with_status(warp::reply::json(&()),warp::http::StatusCode::BAD_REQUEST);

        match contract {
            ContractType::MarketPlace => {
                if let Err(_) = MarketplaceActions::from_str(&action) {
                    return Ok(badreq);
                }
            },
            ContractType::NftShop => {
                // Not implemented
                return Ok(badreq);
            },
            ContractType::NftMinter => {
                // Not implemented
                return Ok(badreq);
            },
            ContractType::TokenMinter => {
                // Not implemented
                return Ok(badreq);
            },
            _ => {
                // Wrong Parameter
                return Ok(badreq);
            }

        }
        let mut client = connect_odin().await;
        let action = ContractAction::from_str(&action).unwrap();
        let cmd = BuildContract::new(customer_id, contract.clone(), action, payload.clone());
        let response = match client.build_cmd::<BuildContract>(cmd).await {
            Ok(ok) => {
                match UnsignedTransaction::from_str(&ok) {
                    Ok(resp) => warp::reply::json(&resp),
                    Err(e)   => warp::reply::json(&ReturnError::new(&e.to_string()))
                }      
            },
            Err(otherwise) => {
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };

        Ok(warp::reply::with_status(
            response, 
            warp::http::StatusCode::OK)
        )

    }

    pub async fn multisig_exec_build(customer_id : u64, multisig_type : MultiSigType, payload : TransactionPattern ) -> Result<impl warp::Reply,Infallible> {
        let badreq = warp::reply::with_status(warp::reply::json(&()),warp::http::StatusCode::BAD_REQUEST);
        log::info!("Build MultiSig Transaction....");
        match multisig_type {
            MultiSigType::SpoRewardClaim => {
            },
            MultiSigType::NftVendor => {
                return Ok(badreq);
            },
            MultiSigType::Mint => {
            },
            MultiSigType::TestRewards => {
            }
            _ => {
                // Wrong Parameter
                return Ok(badreq);
            }
        }

        let mut client = connect_odin().await;    
        let cmd = BuildMultiSig::new(customer_id, multisig_type.clone(), payload.clone());
        let response = match client.build_cmd::<BuildMultiSig>(cmd).await {
            Ok(ok) => {
                match UnsignedTransaction::from_str(&ok) {
                    Ok(resp) => warp::reply::json(&resp),
                    Err(e)   => warp::reply::json(&ReturnError::new(&e.to_string()))
                }     
            },
            Err(otherwise) => {
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };
        Ok(warp::reply::with_status(
           response, 
            warp::http::StatusCode::OK)
        )
    }

    pub async fn stdtx_exec_build(customer_id : u64, tx_type : StdTxType, payload : TransactionPattern ) -> Result<impl warp::Reply,Infallible> {
        let badreq = warp::reply::with_status(warp::reply::json(&()),warp::http::StatusCode::BAD_REQUEST);
        
        match tx_type {
            StdTxType::DelegateStake => {
            },
            _ => {
                // Wrong Parameter
               return Ok(badreq);
            }
        }

        let mut client = connect_odin().await;

    
        let cmd = BuildStdTx::new(customer_id, tx_type.clone(), payload.clone());
              
        let response = match client.build_cmd::<BuildStdTx>(cmd).await {
            Ok(ok) => {
                match UnsignedTransaction::from_str(&ok) {
                    Ok(resp) => warp::reply::json(&resp),
                    Err(e)   => warp::reply::json(&ReturnError::new(&e.to_string()))
                }     
            },
            Err(otherwise) => {
                warp::reply::json(&ReturnError::new(&otherwise.to_string()))
            }
        };
        Ok(warp::reply::with_status(
           response, 
            warp::http::StatusCode::OK)
        )
    }

    pub async fn contract_exec_finalize(customer_id : u64, contract : ContractType,  tx_id : String, payload : Signature) -> Result<impl warp::Reply,Infallible> {
        
         let mut client = connect_odin().await;
         let cmd = FinalizeContract::new(customer_id, contract.clone(), tx_id, payload.get_signature());
         let response = match client.build_cmd(cmd).await {
             Ok(res) => warp::reply::json(&TxHash::new(&res)),
             Err(e) => warp::reply::json(&ReturnError::new(&e.to_string()))
         };
 
        Ok(warp::reply::with_status(
            response, 
            warp::http::StatusCode::OK)
        )
    }

    pub async fn multisig_exec_finalize(customer_id : u64, multisig_type : MultiSigType, tx_id : String, payload : Signature) -> Result<impl warp::Reply,Infallible> {

         let mut client = connect_odin().await;     
         let cmd = FinalizeMultiSig::new(customer_id, multisig_type.clone(), tx_id, payload.get_signature());
         let response = match client.build_cmd(cmd).await {
            Ok(res) => warp::reply::json(&TxHash::new(&res)),
            Err(e) => warp::reply::json(&ReturnError::new(&e.to_string()))
         };
 

        Ok(warp::reply::with_status(
            response, 
            warp::http::StatusCode::OK)
        )
    }

    pub async fn stdtx_exec_finalize(customer_id : u64, txtype : StdTxType, tx_id : String, payload : Signature) -> Result<impl warp::Reply,Infallible> {
        let mut client = connect_odin().await;   
        let cmd = FinalizeStdTx::new(customer_id, txtype.clone(), tx_id, payload.get_signature());
        let response = match client.build_cmd(cmd).await {
           Ok(res) => warp::reply::json(&TxHash::new(&res)),
           Err(e) => warp::reply::json(&ReturnError::new(&e.to_string()))
        };

       Ok(warp::reply::with_status(
           response, 
           warp::http::StatusCode::OK)
       )
   }
}
