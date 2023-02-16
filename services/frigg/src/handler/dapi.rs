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

use crate::error::Error;
use crate::WebResult;
use serde_json::json;
use warp::{reject, Reply};

pub async fn enterprise_create_apikey_post_handler(uid: String) -> WebResult<impl Reply> {
    println!("Create Token for user: {:?}", uid);
    let user = match uid.parse::<i64>() {
        Ok(u) => u,
        Err(_) => return Err(reject::custom(Error::Custom("invalid user".to_string()))),
    };

    let token = sleipnir::apiauth::create_jwt(&user, None)?;
    println!("Api Token: {:?}", token);

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "token": token })),
        warp::http::StatusCode::CREATED,
    ))
}
