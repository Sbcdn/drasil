/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use crate::error::Error;
use crate::WebResult;
use serde::{Deserialize};
use serde_json::{json};
use warp::{reject, Reply};
use super::get_user_from_string;

#[derive(Deserialize,Debug, Clone)]
pub struct CreateContract{
    network: u8,
    contract_fee: Option<i64>,
}

pub async fn entrp_create_sporwc(uid: String, cparam : CreateContract) -> WebResult<impl Reply> {
    let mut net = murin::clib::NetworkIdKind::Mainnet; 
    if cparam.network == 0 {
        net = murin::clib::NetworkIdKind::Testnet; 
    } 

    let user = get_user_from_string(&uid).await?;

    let contract_id = sleipnir::rewards::create_contract(net, user, cparam.contract_fee).await?;
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "contract_id" : contract_id })),
        warp::http::StatusCode::CREATED)
        
    )
}

pub async fn enterprise_get_rwd_contracts_handler(uid: String) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let contracts = sleipnir::rewards::get_rwd_contracts_for_user(user).await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&contracts),
        warp::http::StatusCode::OK)
        
    )
}

#[derive(Deserialize,Debug, Clone)]
pub struct Contract{
    contract_id: i64,
}

pub async fn entrp_depricate_sporwc(uid: String, cparam : Contract) -> WebResult<impl Reply> {

    let user = get_user_from_string(&uid).await?;

    let contract_id = sleipnir::rewards::depricate_contract( user, cparam.contract_id).await?;
    Ok(warp::reply::with_status(
        warp::reply::json(&contract_id),
        warp::http::StatusCode::OK)
        
    )
}

pub async fn entrp_reactivate_sporwc(uid: String, cparam : Contract) -> WebResult<impl Reply> {

    let user = get_user_from_string(&uid).await?;

    let contract_id = sleipnir::rewards::reactivate_contract( user, cparam.contract_id).await?;
    Ok(warp::reply::with_status(
        warp::reply::json(&contract_id),
        warp::http::StatusCode::OK)
        
    )
}


pub async fn get_contract_tokens(
    uid: String,
    cparam : Contract
) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;
    let tokens = sleipnir::rewards::get_tokens_from_contract(user, cparam.contract_id).await?;
    
    Ok(warp::reply::with_status(
        warp::reply::json(&tokens),
        warp::http::StatusCode::OK)
        
    )
}


#[derive(Deserialize,Debug, Clone)]
pub struct AddTokenWhitelisitng {
    contract_id         : i64,
    fingerprint         : String,
    vesting_period      : Option<String>,
    pools               : Option<Vec::<String>>,
    mode                : String,
    equation            : String,
    start_epoch         : i64,
    end_epoch           : Option<i64>,
    modificator_equ     : Option<String>
}

pub async fn entrp_add_token_sporwc(uid: String, cparam : AddTokenWhitelisitng) -> WebResult<impl Reply> { 
    let user = get_user_from_string(&uid).await?;    
    let _ = match hugin::database::TBContracts::get_contract_uid_cid(user, cparam.contract_id) {
        Ok(c) => {
            if c.contract_type != "sporwc".to_string() {
                return Err(reject::custom(Error::Custom("error in requested contract update, contract has wrong type".to_string())))    
            }
        }
        Err(_) => {
            return Err(reject::custom(Error::Custom("error in requested contract update, contract does not exist".to_string())))
        } 
    };
    log::debug!("Try to create TokenWhitelisting...");
    let token_listing = sleipnir::rewards::create_token_whitelisting(
        user, 
        cparam.contract_id, 
        cparam.fingerprint, 
        cparam.vesting_period, 
        cparam.pools, 
        cparam.mode, 
        cparam.equation, 
        cparam.start_epoch, 
        cparam.end_epoch, 
        cparam.modificator_equ
    ).await?;
    
    Ok(warp::reply::with_status(
        warp::reply::json(&token_listing), //
        warp::http::StatusCode::CREATED)
        
    )
     
}

#[derive(Deserialize,Debug, Clone)]
pub struct GetTWL {
    contract_id         : i64,
    fingerprint         : String,
}

pub async fn entrp_rm_token_sporwc(uid: String, cparam : GetTWL) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;
    let _ = match hugin::database::TBContracts::get_contract_uid_cid(user, cparam.contract_id) {
        Ok(c) => {
            if c.contract_type != "sporwc".to_string() {
                return Err(reject::custom(Error::Custom("error in requested contract update, contract has wrong type".to_string())))    
            }
        }
        Err(_) => {
            return Err(reject::custom(Error::Custom("error in requested contract update, contract does not exist".to_string())))
        } 
    };
    
    let token_listing = sleipnir::rewards::remove_token_whitelisting(
        user, 
        cparam.contract_id, 
        cparam.fingerprint, 
    ).await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&token_listing),
        warp::http::StatusCode::OK)
        
    )
}


pub async fn get_pools(uid: String, cparam : GetTWL) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let resp = sleipnir::rewards::get_pools(
        user,
        cparam.contract_id,
        cparam.fingerprint,
    ).await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&resp),
        warp::http::StatusCode::OK)
        
    )
}

#[derive(Deserialize,Debug, Clone)]
pub struct TxCountStat {
    contract_id : Option<String>,
    from        : Option<String>,
    to          : Option<String>,
}

pub async fn get_user_txs(uid: String, cparam : TxCountStat) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let resp = sleipnir::rewards::get_user_txs(
        user,
        cparam.from,
        cparam.to,
    ).await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({"tx_count" : resp })),
        warp::http::StatusCode::OK)
        
    )
}

#[derive(Deserialize,Debug, Clone)]
pub struct AddPools {
    contract_id         : i64,
    fingerprint         : String,
    pools               : Vec::<String>,
}

pub async fn add_pools(uid: String, cparam : AddPools) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let resp = sleipnir::rewards::add_pools(
        user,
        cparam.contract_id,
        cparam.fingerprint,
        cparam.pools,
    ).await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&resp),
        warp::http::StatusCode::CREATED)
        
    )
}


#[derive(Deserialize,Debug, Clone)]
pub struct RmPools {
    contract_id         : i64,
    fingerprint         : String,
    pools               : Vec::<String>,
}

pub async fn remove_pools(uid: String, cparam : RmPools) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let resp = sleipnir::rewards::rm_pools(
        user,
        cparam.contract_id,
        cparam.fingerprint,
        cparam.pools,
    ).await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&resp),
        warp::http::StatusCode::OK)
        
    )
}