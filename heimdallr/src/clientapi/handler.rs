/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use hugin::{BuildContract, BuildMultiSig,FinalizeContract,FinalizeMultiSig, BuildStdTx, FinalizeStdTx};
use hugin::datamodel::hephadata::{ 
    TransactionPattern,OneShotMintPayload,MultiSigType,UnsignedTransaction, ReturnError, OneShotReturn
};
use hugin::client::{Client, connect};
use std::{convert::Infallible, str::FromStr};
use std::env;

async fn connect_odin() -> Client {
    connect(env::var("ODIN_URL").unwrap()).await.unwrap()
}

pub async fn hnd_oneshot_minter_api(customer_id : u64, payload : OneShotMintPayload ) -> Result<impl warp::Reply,Infallible> {
    let badreq = warp::reply::with_status(warp::reply::json(&()),warp::http::StatusCode::BAD_REQUEST);
    log::info!("Build Oneshot Minter Transaction....");

    if payload.tokennames().len() != payload.amounts().len() {
        return Ok(badreq)
    }

    let multisig_type = MultiSigType::ClAPIOneShotMint;
    let transaction_pattern = TransactionPattern::new_empty(
        customer_id,
        &payload.into_script_spec(),
        payload.network(),
    );

    let mut client = connect_odin().await;    
    let cmd = BuildMultiSig::new(customer_id, multisig_type.clone(), transaction_pattern);         
    let response = match client.build_cmd::<BuildMultiSig>(cmd).await {
        Ok(ok) => {
            match serde_json::from_str::<OneShotReturn>(&ok) {
                Ok(resp ) => warp::reply::json(&resp),
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