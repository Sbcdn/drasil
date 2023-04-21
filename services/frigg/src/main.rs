#![recursion_limit = "256"]
#![allow(opaque_hidden_inferred_bound)]
/*
#################################################################################
# Business Source License           See LICENSE.md for full license information.#
# Licensor:             Drasil Blockchain Association                           #
# Licensed Work:        Drasil Application Framework v.0.2. The Licensed Work   #
#                       is © 2022 Drasil Blockchain Association                 #
# Additional Use Grant: You may use the Licensed Work when your application     #
#                       using the Licensed Work is generating less than         #
#                       $150,000 and the entity operating the application       #
#                       engaged equal or less than 10 people.                   #
# Change Date:          Drasil Application Framework v.0.2, change date is two  #
#                       and a half years from release date.                     #
# Change License:       Version 2 or later of the GNU General Public License as #
#                       published by the Free Software Foundation.              #
#################################################################################
*/

extern crate pretty_env_logger;

extern crate log;

pub mod handler;

use auth::{with_auth, Role};
use deadpool_lapin::Pool;
use error::Error::*;
use handler::{
    rwd::{Contract, GetTWL, TxCountStat},
    Clients,
};
use lapin::ConnectionProperties;
use nonzero_ext::nonzero;
use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tokio::sync::Mutex;
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

fn with_rmq(pool: Pool) -> impl Filter<Extract = (Pool,), Error = Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

fn endpoints(
    _clients: Clients,
    pool: Pool,
    _rate_limiter: DirectRateLimiter<LeakyBucket>,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + std::clone::Clone + 'static
{
    let login_route = warp::path!("login")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(login_handler);

    let register_route = warp::path!("register")
        .and(warp::post())
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

    // get set pool in a contract
    let enterprise_get_user_tx = enterprise_get
        .clone()
        .and(warp::path("ms"))
        .and(warp::path("stats"))
        .and(warp::path("sprwc"))
        .and(warp::path("tx"))
        //.and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::get_user_txs_all);

    // get set pool in a contract
    let enterprise_get_user_tx_timed = enterprise_get
        .clone()
        .and(warp::path("ms"))
        .and(warp::path("stats"))
        .and(warp::path("sprwc"))
        .and(warp::path("tx"))
        .and(warp::path("timed"))
        .and(warp::path::end())
        .and(warp::query::<TxCountStat>())
        //.and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::get_user_txs_timed);

    // get all availabale contracts
    let enterprise_get_contracts = enterprise_get
        .clone()
        .and(warp::path("rwd"))
        .and(warp::path("contr"))
        .and(warp::path::end())
        .and_then(handler::rwd::enterprise_get_rwd_contracts_handler);

    // get set pool in a contract
    let enterprise_get_pools = enterprise_get
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("pools"))
        .and(warp::path::end())
        .and(warp::query::<GetTWL>())
        //.and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::get_pools);

    // get set pool in a contract
    let enterprise_get_contract_tokens = enterprise_get
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("tokens"))
        .and(warp::path::end())
        .and(warp::query::<Contract>())
        //.and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::get_contract_tokens);

    let ent_get = enterprise_create_api_token
        .or(enterprise_get_user_tx)
        .or(enterprise_get_user_tx_timed)
        .or(enterprise_get_contracts)
        .or(enterprise_get_pools)
        .or(enterprise_get_contract_tokens);

    // Enterprise POST

    // Create discount for contract
    let enterprise_post_create_discount = enterprise_post
        .clone()
        .and(warp::path("disco"))
        .and(warp::path("cr"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::discounts::hndl_create_discount);

    // Create discount for contract
    let enterprise_post_remove_discount = enterprise_post
        .clone()
        .and(warp::path("disco"))
        .and(warp::path("rm"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::discounts::hndl_remove_discount);

    // Allocate Whitelist to an NFT project
    let enterprise_post_rnd_alloc_nfts_to_mp = enterprise_post
        .clone()
        .and(warp::path("mint"))
        .and(warp::path("alloc"))
        .and(warp::path("rnd"))
        .and(with_rmq(pool.clone()))
        .and(warp::body::content_length_limit(10000 * 1024).and(warp::body::json()))
        .and_then(handler::whitelist::random_allocate_whitelist_to_mp);

    // Allocate Whitelist to an NFT project
    let enterprise_post_alloc_nfts_to_mp = enterprise_post
        .clone()
        .and(warp::path("mint"))
        .and(warp::path("alloc"))
        .and(warp::path("sa"))
        .and(with_rmq(pool.clone()))
        .and(warp::body::content_length_limit(10000 * 1024).and(warp::body::json()))
        .and_then(handler::whitelist::allocate_whitelist_to_mp);

    // Import NFTs via CIP25 metadata
    let enterprise_post_import_nfts_csv_meta = enterprise_post
        .clone()
        .and(warp::path("mint"))
        .and(warp::path("impcsv"))
        .and(with_rmq(pool.clone()))
        .and(warp::body::content_length_limit(10000 * 1024).and(warp::body::json()))
        .and_then(handler::mint::entrp_create_nfts_from_csv);

    // Import NFTs via CIP25 metadata
    let enterprise_post_import_nfts_csv_meta_2 = enterprise_post
        .clone()
        .and(warp::path("mint"))
        .and(warp::path("simpcsv"))
        .and(warp::path::param::<i64>())
        .and(warp::body::content_length_limit(10000 * 1024).and(warp::body::bytes()))
        .and_then(handler::mint::entrp_create_nfts_from_csv_s);

    // Reactivate a Depricated Reward Contract
    let enterprise_post_reactivate_reward_contract = enterprise_post
        .clone()
        .and(warp::path("ms"))
        .and(warp::path("act"))
        .and(warp::path("sprwc"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::entrp_reactivate_sporwc);

    // Create a new mint project
    let enterprise_post_create_mint_project = enterprise_post
        .clone()
        .and(warp::path("ms"))
        .and(warp::path("cr"))
        .and(warp::path("cmint"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::mint::entrp_create_mint_proj);

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

    // Add a pool to a Whitelistes Token
    let enterprise_post_add_pools = enterprise_post
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("addpools"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::add_pools);

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

    // Remove a pool from a Whitelisted Token
    let enterprise_post_rm_pools = enterprise_post
        .clone()
        .and(warp::path("sprwc"))
        .and(warp::path("rmpools"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::rwd::remove_pools);

    // Create a new reward contract
    let enterprise_post_create_lqdt_wallet = enterprise_post
        .clone()
        .and(warp::path("wal"))
        .and(warp::path("cr"))
        .and(warp::path("lqdt"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::adm::adm_create_lqdt);

    // Create an empty whitelist
    let enterprise_post_create_whitelist = enterprise_post
        .clone()
        .and(warp::path("ws"))
        .and(warp::path("cr"))
        .and(warp::body::content_length_limit(1000 * 1024).and(warp::body::json()))
        .and_then(handler::whitelist::create_whitelist);

    // Delete a whitelist and all its content
    let enterprise_post_delete_whitelist = enterprise_post
        .clone()
        .and(warp::path("ws"))
        .and(warp::path("rm"))
        .and(warp::body::content_length_limit(1000 * 1024).and(warp::body::json()))
        .and_then(handler::whitelist::delete_whitelist);

    // Import Whitelist
    let enterprise_post_import_whitelist = enterprise_post
        .clone()
        .and(warp::path("ws"))
        .and(warp::path("impcsv"))
        .and(with_rmq(pool.clone()))
        .and(warp::body::content_length_limit(10000 * 1024).and(warp::body::json()))
        .and_then(handler::whitelist::import_whitelist_from_csv);

    let ent_post = enterprise_post_create_discount
        .or(enterprise_post_remove_discount)
        .or(enterprise_post_rnd_alloc_nfts_to_mp)
        .or(enterprise_post_alloc_nfts_to_mp)
        .or(enterprise_post_import_nfts_csv_meta)
        .or(enterprise_post_import_nfts_csv_meta_2)
        .or(enterprise_post_reactivate_reward_contract)
        .or(enterprise_post_create_mint_project)
        .or(enterprise_post_create_reward_contract)
        .or(enterprise_post_deprecate_reward_contract)
        .or(enterprise_post_add_pools)
        .or(enterprise_post_add_token_sporwc)
        .or(enterprise_post_rm_token_sporwc)
        .or(enterprise_post_rm_pools)
        .or(enterprise_post_create_lqdt_wallet)
        .or(enterprise_post_create_whitelist)
        .or(enterprise_post_delete_whitelist)
        .or(enterprise_post_import_whitelist);

    // Endpoint Accumulators
    let enterprise = ent_get.or(ent_post);

    // Retailer Routes

    let retailer_route = warp::path("ret").and(with_auth(Role::Retailer));

    let _retailer_get = retailer_route.clone().and(warp::get());

    //let retailer_post = retailer_route.clone().and(warp::post());

    // Drasil Admin Routes

    let admin_route = warp::path("adm").and(with_auth(Role::DrasilAdmin));

    let adm_get = admin_route.clone().and(warp::get());

    let adm_post = admin_route.clone().and(warp::post());

    let adm_create_payout = adm_post
        .clone()
        .and(warp::path("po"))
        .and(warp::path("cr"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::adm::adm_create_payout);

    let adm_exec_payout = adm_post
        .clone()
        .and(warp::path("po"))
        .and(warp::path("ex"))
        .and(warp::body::content_length_limit(100 * 1024).and(warp::body::json()))
        .and_then(handler::adm::adm_execute_payout);

    let adm_list_payouts = adm_get
        .clone()
        .and(warp::path("po"))
        .and(warp::path("list"))
        .and_then(handler::adm::adm_list_payouts);

    let admin = adm_create_payout.or(adm_exec_payout).or(adm_list_payouts);

    // Routes
    login_route
        .or(register_route)
        .or(verify_email_route)
        .or(enterprise)
        .or(retailer_route)
        .or(admin)
        .or(user)
}

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    let host: String = std::env::var("POD_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port = std::env::var("POD_PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string());

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

    // RMQ
    let manager = deadpool_lapin::Manager::new(
        handler::AMQP_ADDR.to_string(),
        ConnectionProperties::default(),
    );
    let pool: deadpool_lapin::Pool = deadpool::managed::Pool::builder(manager)
        .max_size(100)
        .build()
        .expect("can't create pool");

    //Rate Limitation
    // Allow 3 units/second across all threads:
    let lim =
        DirectRateLimiter::<LeakyBucket>::new(nonzero!(2u32), std::time::Duration::from_secs(5));

    // Warp-Server
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

    let routes = endpoints(clients, pool, lim)
        .recover(error::handle_rejection)
        .with(cors)
        .with(warp::log("frigg"));

    let server = host.to_string() + ":" + &port;
    let socket: std::net::SocketAddr = server.parse().expect("Unable to parse socket address");
    warp::serve(routes).run(socket).await;
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
    let new_user = TBDrasilUser::create_user(
        None,
        &payload.username,
        &payload.email,
        &payload.pwd,
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
    .await
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
    Ok(format!("Hello User {uid}"))
}

pub async fn enterprise_post_handler(
    uid: String,
    _param: String,
    _json: String,
) -> WebResult<impl Reply> {
    Ok(format!("Hello Enterprise {uid}"))
}

pub async fn enterprise_get_handler(uid: String, param: String) -> WebResult<impl Reply> {
    Ok(format!("Hello Enterprise {uid}, p: {param}"))
}

pub async fn retailer_handler(uid: String) -> WebResult<impl Reply> {
    Ok(format!("Hello Retailer {uid}"))
}

pub async fn admin_handler(uid: String) -> WebResult<impl Reply> {
    Ok(format!("Hello Admin {uid}"))
}
