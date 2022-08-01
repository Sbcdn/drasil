/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use super::handler;
use hugin::datamodel::hephadata::OneShotMintPayload;
use warp::Filter;

pub fn api_endpoints() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    resp_option().or(oneshot_minter_api())
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

pub fn oneshot_minter_api(
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("api")
        .and(warp::path("mint"))
        .and(warp::path("oneshot"))
        .and(warp::post())
        .and(auth())
        .and(json_oneshot_minter_body())
        .and_then(handler::hnd_oneshot_minter_api)
}

fn json_oneshot_minter_body(
) -> impl Filter<Extract = (OneShotMintPayload,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(100 * 1024).and(warp::body::json())
}

fn auth() -> impl Filter<Extract = (u64,), Error = warp::Rejection> + Clone {
    use crate::auth::authorize;
    use warp::{
        filters::header::headers_cloned,
        http::header::{HeaderMap, HeaderValue},
    };
    headers_cloned()
        .map(move |headers: HeaderMap<HeaderValue>| (headers))
        .and_then(authorize)
}
