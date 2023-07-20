use crate::error::Error;
use crate::WebResult;
use serde_json::json;
use warp::{reject, Reply};

pub async fn enterprise_create_apikey_post_handler(uid: String) -> WebResult<impl Reply> {
    log::debug!("Create Token for user: {:?}", uid);
    let user = match uid.parse::<i64>() {
        Ok(u) => u,
        Err(_) => return Err(reject::custom(Error::Custom("invalid user".to_string()))),
    };

    let token = sleipnir::apiauth::create_jwt(&user, None)?;
    log::debug!("Api Token: {:?}", token);

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "token": token })),
        warp::http::StatusCode::CREATED,
    ))
}
