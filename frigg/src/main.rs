/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

extern crate pretty_env_logger;

extern crate log;

pub mod handler;

use auth::{with_auth, Role};
use error::Error::*;
use serde::{Deserialize, Serialize};
use std::env;
use warp::{reject, reply, Filter, Rejection, Reply};

use hugin::drasildb::TBDrasilUser;

mod auth;
mod email_verify;
mod error;

type Result<T> = std::result::Result<T, error::Error>;
type WebResult<T> = std::result::Result<T, Rejection>;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub pw: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

// ToDo: ORT fehlt
#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    username: String,
    email: String,
    pwd: String,
    company_name: Option<String>,
    address: Option<String>,
    post_code: Option<String>,
    city: Option<String>,
    addional_addr: Option<String>,
    country: Option<String>,
    contact_p_fname: Option<String>,
    contact_p_sname: Option<String>,
    contact_p_tname: Option<String>,
    cardano_wallet: Option<String>,
}

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "8000";

#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }

    //let cli = Cli::from_args();
    let host: String = env::var("POD_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string()); //cli.host.as_deref().unwrap_or(DEFAULT_HOST);
    let port = env::var("POD_PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string()); //cli.port.as_deref().unwrap_or(DEFAULT_PORT);

    let login_route = warp::path!("login")
        .and(warp::post())
        // .and(with_users(users.clone()))
        .and(warp::body::json())
        .and_then(login_handler);

    let register_route = warp::path!("register")
        .and(warp::post())
        // .and(with_users(users.clone()))
        .and(warp::body::json())
        .and_then(register_handler);

    let verify_email_route = warp::path!("verema")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(verify_email);

    // Standard User Routes

    let user_route = warp::path("use").and(with_auth(Role::StandardUser));

    // get DrasilUser profile
    let user_get_profile = user_route
        .clone()
        .and(warp::path("profile"))
        .and(warp::path::param::<String>())
        .and_then(enterprise_get_handler);

    let user = user_get_profile;

    // Enterprise Routes

    let enterprise_route = warp::path("ent").and(with_auth(Role::EnterpriseUser));

    let enterprise_get = enterprise_route.clone().and(warp::get());

    let enterprise_post = enterprise_route.clone().and(warp::post());

    let enterprise_create_api_token = enterprise_get
        .clone()
        .and(warp::get())
        .and(warp::path("api"))
        .and(warp::path("cr"))
        .and_then(handler::dapi::enterprise_create_apikey_post_handler);

    // get all availabale contracts
    let enterprise_get_contracts = enterprise_get
        .clone()
        .and(warp::path("rwd"))
        .and(warp::path("contr"))
        .and_then(handler::rwd::enterprise_get_rwd_contracts_handler);

    // get set pool in a contract
    let enterprise_get_pools = enterprise_get
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("pools"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::get_pools);

    // get set pool in a contract
    let enterprise_get_user_tx = enterprise_get
        .clone()
        .and(warp::path("ms"))
        .and(warp::path("stats"))
        .and(warp::path("sprwc"))
        .and(warp::path("tx"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::get_user_txs);

    // get set pool in a contract
    let enterprise_get_contract_tokens = enterprise_get
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("tokens"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::get_contract_tokens);

    // Enterprise POST

    // Create a new reward contract
    let enterprise_post_create_reward_contract = enterprise_post
        .clone()
        .and(warp::path("ms"))
        .and(warp::path("cr"))
        .and(warp::path("sprwc"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::entrp_create_sporwc);

    // Deactivate a Reward Contract (set to depricated)
    let enterprise_post_deprecate_reward_contract = enterprise_post
        .clone()
        .and(warp::path("ms"))
        .and(warp::path("depr"))
        .and(warp::path("sprwc"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::entrp_depricate_sporwc);

    // Add a Token to a contract (whitelist a token)
    let enterprise_post_add_token_sporwc = enterprise_post
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("addt"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::entrp_add_token_sporwc);

    // Remove a TOken from a Contract (Remove from Whitelist)
    let enterprise_post_rm_token_sporwc = enterprise_post
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("rmt"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::entrp_rm_token_sporwc);

    // Add a pool to a Whitelistes Token
    let enterprise_post_add_pools = enterprise_post
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("addpools"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::add_pools);

    // Remove a pool from a Whitelisted Token
    let enterprise_post_rm_pools = enterprise_post
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("rmpools"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::remove_pools);

    // Endpoint Accumulators
    let pools = enterprise_get_pools
        .or(enterprise_post_add_pools)
        .or(enterprise_post_rm_pools);

    let sporwc = enterprise_post_create_reward_contract
        .or(enterprise_post_deprecate_reward_contract)
        .or(enterprise_get_contract_tokens)
        .or(enterprise_post_add_token_sporwc)
        .or(enterprise_post_rm_token_sporwc)
        .or(enterprise_get_user_tx);

    let enterprise = sporwc
        .or(pools)
        .or(enterprise_create_api_token)
        .or(enterprise_get_contracts);

    // Retailer Routes

    let retailer_route = warp::path("ret").and(with_auth(Role::Retailer));

    let _retailer_get = retailer_route.clone().and(warp::get());

    let retailer_post = retailer_route.clone().and(warp::post());

    // Reactivate a Depricated Reward Contract
    let enterprise_post_reactivate_reward_contract = retailer_post
        .clone()
        .and(warp::path("ms"))
        .and(warp::path("act"))
        .and(warp::path("sprwc"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::entrp_reactivate_sporwc);

    let _retailer = enterprise_post_reactivate_reward_contract;

    // Drasil Admin Routes

    let admin_route = warp::path("adm")
        .and(with_auth(Role::DrasilAdmin))
        .and_then(admin_handler);

    // Routes

    let endpoints = login_route
        .or(register_route)
        .or(verify_email_route)
        .or(user)
        .or(enterprise)
        .or(retailer_route)
        .or(admin_route)
        .or(warp::get().and(warp::any().map(warp::reply)))
        .recover(error::handle_rejection);

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

    pretty_env_logger::init();

    // Warp-Server
    let api = endpoints;
    // view access logs by setting RUST_LOG=hepha
    let routes = api.with(cors).with(warp::log("frigg"));

    let server = host.to_string() + ":" + &port;
    let socket: std::net::SocketAddr = server.parse().expect("Unable to parse socket address");

    //dotenv::dotenv().ok();
    //let cert_path = env::var("TLS_CERT_PATH").unwrap();
    //let key_path = env::var("TLS_KEY_PATH").unwrap();

    warp::serve(routes).run(socket).await; //

    //if host == "127.0.0.1" {
    //    warp::serve(routes).run(socket).await; //
    //} else {
    //    warp::serve(routes).tls().cert_path(Path::new(&cert_path)).key_path(Path::new(&key_path)).run(socket).await; //
    //}
}

pub async fn login_handler(body: LoginRequest) -> WebResult<impl Reply> {
    let user = TBDrasilUser::verify_pw_user(&body.email, &body.pw);
    match user {
        Ok(u) => match u.email_verified {
            true => {
                let token = auth::create_jwt(&u.user_id.to_string(), &Role::from_str(&u.role))
                    .map_err(reject::custom)?;
                Ok(reply::json(&LoginResponse { token }))
            }
            _ => Err(reject::custom(error::Error::EmailNotVerified)),
        },
        Err(_) => Err(reject::custom(WrongCredentialsError)),
    }
}

pub async fn register_handler(payload: RegisterRequest) -> WebResult<impl Reply> {
    let conn = hugin::drasildb::establish_connection()
        .map_err(|_| error::Error::Custom("Could not establish database connection".to_string()))?;
    log::info!("Payload {:?}", payload);
    let new_user = TBDrasilUser::create_user(
        &conn,
        None,
        &payload.username,
        &payload.email,
        &payload.pwd,
        &Role::StandardUser.to_string(),
        &Vec::<String>::new(),
        payload.company_name.as_ref(),
        payload.address.as_ref(),
        payload.post_code.as_ref(),
        payload.city.as_ref(),
        payload.addional_addr.as_ref(),
        payload.country.as_ref(),
        payload.contact_p_fname.as_ref(),
        payload.contact_p_sname.as_ref(),
        payload.contact_p_tname.as_ref(),
        &Vec::<String>::new(),
        payload.cardano_wallet.as_ref(),
    )
    .map_err(|e| error::Error::Custom(format!("Could not create new user: {:?}", e.to_string())))?;

    // Send verification Email to [new_user.email]
    let email_body = hugin::database::TBEmailVerificationTokenMessage::new(
        Some(new_user.uname.clone()),
        &new_user.email,
    );
    let resp = email_verify::invite(email_body).await?;

    Ok(resp)
}

pub async fn verify_email(payload: email_verify::RegistrationMessage) -> WebResult<impl Reply> {
    let emv = email_verify::verify(payload).await?;
    Ok(emv)
}

pub async fn user_handler(uid: String) -> WebResult<impl Reply> {
    Ok(format!("Hello User {}", uid))
}

pub async fn enterprise_post_handler(
    uid: String,
    _param: String,
    _json: String,
) -> WebResult<impl Reply> {
    Ok(format!("Hello Enterprise {}", uid))
}

pub async fn enterprise_get_handler(uid: String, param: String) -> WebResult<impl Reply> {
    Ok(format!("Hello Enterprise {}, p: {}", uid, param))
}

pub async fn retailer_handler(uid: String) -> WebResult<impl Reply> {
    Ok(format!("Hello Retailer {}", uid))
}

pub async fn admin_handler(uid: String) -> WebResult<impl Reply> {
    Ok(format!("Hello Admin {}", uid))
}
