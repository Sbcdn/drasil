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


pub async fn enterprise_create_apikey_post_handler(uid: String) -> WebResult<impl Reply> {
    println!("Create Token for user: {:?}", uid);
    let user = match uid.parse::<i64>() {
        Ok(u) => u,
        Err(_) => {
            return Err(reject::custom(Error::Custom("invalid user".to_string())))
        }
    };

    let token = sleipnir::apiauth::create_jwt(&user, None)?;
    println!("Api Token: {:?}",token);

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "token" : token })),
        warp::http::StatusCode::CREATED)
        
    )
}