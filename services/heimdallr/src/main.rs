#![allow(opaque_hidden_inferred_bound, unused_imports)]
extern crate pretty_env_logger;

use std::env;

use heimdallr::clientapi;
use warp::Filter;

/// The default socket address exposed by Heimdallr server unless another address is specified. 
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "4000";

/// Run Heimdallr server on the given socket address, with the given filters.
#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }

    let host: String = env::var("POD_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port = env::var("POD_PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string());

    pretty_env_logger::init();

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

    let api = clientapi::endpoints();
    let routes = api.with(cors).with(warp::log("heimdallr"));
    let server = format!("{host}:{port}");
    let socket: std::net::SocketAddr = server.parse().expect("Unable to parse socket address");
    warp::serve(routes).run(socket).await;
}
