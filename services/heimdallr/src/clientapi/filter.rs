use super::handler;
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
        .and(crate::filters::auth())
        .and_then(handler::hnd_oneshot_minter_api)
}
