#![allow(opaque_hidden_inferred_bound)]
extern crate pretty_env_logger;

use std::{env, path::Path};

use drasil_dvltath::error::Error;
use lazy_static::lazy_static;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use warp::Filter;

pub type Result<T> = std::result::Result<T, Error>;

lazy_static! {
    static ref SOCKET_PATH: String =
        std::env::var("VSOCKET_PATH").unwrap_or_else(|_| "./warp.sock".to_string());
    static ref OROLE_ID: String =
        std::env::var("OROLE_ID").unwrap_or_else(|_| "role-id".to_string());
    static ref OSECRET_ID: String =
        std::env::var("OSECRET_ID").unwrap_or_else(|_| "secret-id".to_string());
}

#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "dvltath=info");
    }
    pretty_env_logger::init();

    let cors2 = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET"])
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

    let api = filters::endpoints();
    let routes = api.with(cors2).with(warp::log("dvlt"));

    let url = SOCKET_PATH.to_string();
    let path = Path::new(&url);

    if path.exists() {
        std::fs::remove_file(path).expect("could not remove file");
    }

    let listener = UnixListener::bind(path).unwrap();
    let incoming = UnixListenerStream::new(listener);
    warp::serve(routes).run_incoming(incoming).await;
}

mod filters {
    use crate::handlers;
    use warp::Filter;
    use warp::{
        http::header::{HeaderMap, HeaderValue},
        Rejection,
    };

    pub fn endpoints() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_secret().or(resp_option())
    }

    pub fn resp_option() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        warp::options()
            .and(warp::header("origin"))
            .map(|origin: String| {
                Ok(warp::http::Response::builder()
                    .status(warp::http::StatusCode::OK)
                    .header("access-control-allow-methods", "GET")
                    .header("access-control-allow-headers", "Authorization")
                    .header("access-control-allow-credentials", "true")
                    .header("access-control-max-age", "300")
                    .header("access-control-allow-origin", origin)
                    .header("vary", "origin")
                    .body(""))
            })
    }

    pub fn get_secret() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        warp::path("auth")
            .and(warp::get())
            .and(auth())
            .and(warp::path::param::<String>())
            .and_then(handlers::handle_get_secret)
    }

    fn auth() -> impl Filter<Extract = ((),), Error = warp::Rejection> + Clone {
        use warp::filters::header::headers_cloned;
        headers_cloned()
            .map(move |headers: HeaderMap<HeaderValue>| (headers))
            .and_then(authorize)
    }

    pub(crate) async fn authorize(headers: HeaderMap<HeaderValue>) -> Result<(), Rejection> {
        log::info!("{:?}", headers);
        Ok(())
    }
}

mod handlers {
    use std::convert::Infallible;
    pub async fn handle_get_secret(_: (), role_id: String) -> Result<impl warp::Reply, Infallible> {
        drasil_dvltath::vault::auth::store_wrapped_secret(&role_id).await;
        Ok(warp::reply::with_status(
            role_id,
            warp::http::StatusCode::ACCEPTED,
        ))
    }
}
