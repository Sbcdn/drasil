/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use super::error::Error;
use super::handlers;
use super::models::{Clients, ErrorResult};
use deadpool_lapin::Pool;
use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
use std::convert::Infallible;
use warp::{hyper::StatusCode, Filter};

pub fn endpoints(
    clients: Clients,
    pool: &Pool,
    rate_limiter: DirectRateLimiter<LeakyBucket>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    websocket(clients, pool.clone(), rate_limiter)
        .or(ok())
        .or(resp_option())
}

pub fn resp_option() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
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

pub fn websocket(
    clients: Clients,
    pool: Pool,
    rate_limiter: DirectRateLimiter<LeakyBucket>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("auth")
        .and(auth())
        .and(warp::ws())
        .and(with_clients(clients))
        .and(with_rmq(pool))
        .and(with_limiter(rate_limiter))
        .and_then(handlers::handle_ws_client)
}

fn with_rmq(pool: Pool) -> impl Filter<Extract = (Pool,), Error = Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

fn with_limiter(
    limiter: DirectRateLimiter<LeakyBucket>,
) -> impl Filter<Extract = (DirectRateLimiter<LeakyBucket>,), Error = Infallible> + Clone {
    warp::any().map(move || limiter.clone())
}

fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

/// Reply 200
pub fn ok() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("alive")
        .and(warp::get())
        .and(warp::any().map(warp::reply))
}

pub(crate) fn auth() -> impl Filter<Extract = (u64,), Error = warp::Rejection> + Clone {
    use super::auth::authorize;
    use warp::{
        filters::header::headers_cloned,
        http::header::{HeaderMap, HeaderValue},
    };
    headers_cloned()
        .map(move |headers: HeaderMap<HeaderValue>| (headers))
        //.and(bytes().map(move |body: bytes::Bytes| (body)))
        .and_then(authorize)
}

pub(crate) async fn handle_rejection(
    err: warp::reject::Rejection,
) -> std::result::Result<impl warp::reply::Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not found";
    } else if err
        .find::<warp::filters::body::BodyDeserializeError>()
        .is_some()
    {
        code = StatusCode::BAD_REQUEST;
        message = "Invalid Body";
    } else if let Some(e) = err.find::<Error>() {
        match e {
            Error::JWTTokenError => {
                code = StatusCode::BAD_GATEWAY;
                message = "Action not authorized";
            }
            Error::NoAuthHeaderError => {
                code = StatusCode::BAD_REQUEST;
                message = "No authentication";
            }
            Error::InvalidAuthHeaderError => {
                code = StatusCode::BAD_REQUEST;
                message = "Invalid authentication";
            }
            Error::Custom(_) => {
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal Error";
            }
            // Error::JWTTokenCreationError => {
            //     code = StatusCode::INTERNAL_SERVER_ERROR;
            //     message = "Token Creation Error";
            // }
            Error::RMQError(_) => {
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "rmq error";
            }
            Error::RMQPoolError(_) => {
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "rmq pool error";
            }
            Error::RateLimitReachedError => {
                code = StatusCode::TOO_MANY_REQUESTS;
                message = "too many requests";
            }
        }
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "Method not allowed";
    } else {
        // We should have expected this... Just log and say its a 500
        log::error!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal server error";
    }

    let json = warp::reply::json(&ErrorResult {
        detail: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}
