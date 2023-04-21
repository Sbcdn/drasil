/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
#![allow(opaque_hidden_inferred_bound)]
extern crate pretty_env_logger;

mod auth;
mod error;
mod filters;
mod handlers;
mod models;

use deadpool_lapin::Pool;
use lapin::ConnectionProperties;
use models::Clients;
use std::env;
use std::{collections::HashMap, str, sync::Arc};
use tokio::sync::Mutex;
use warp::{Filter, Rejection};

use nonzero_ext::nonzero;
use ratelimit_meter::{DirectRateLimiter, LeakyBucket};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "40001";

#[tokio::main]
async fn main() -> Result<(), error::Error> {
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

    // RMQ
    let addr =
        std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rmq:rmq@127.0.0.1:5672/%2f".into());
    let manager = deadpool_lapin::Manager::new(addr, ConnectionProperties::default());
    let pool: deadpool_lapin::Pool = deadpool::managed::Pool::builder(manager)
        .max_size(100)
        .build()
        .expect("can't create pool");
    log::info!("pool: {pool:?}");
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

    //Rate Limitation
    // Allow 3 units/second across all threads:
    let lim =
        DirectRateLimiter::<LeakyBucket>::new(nonzero!(2u32), std::time::Duration::from_secs(5));
    let api = filters::endpoints(clients.clone(), &pool, lim);

    log::info!("Configuring websocket route");
    let routes = api
        .with(cors)
        .recover(filters::handle_rejection)
        .with(warp::log("loki"));
    log::info!("Starting update loop");
    tokio::task::spawn(async move {
        handlers::main_worker(clients.clone()).await;
    });
    log::info!("Starting server");

    let server = host.clone() + ":" + &port;
    let socket: std::net::SocketAddr = server.parse().expect("Unable to parse socket address");

    let _ = futures::join!(warp::serve(routes).run(socket)); // rmq_listen(pool)
    Ok(())
}

async fn get_rmq_con(pool: Pool) -> Result<models::Connection, deadpool_lapin::PoolError> {
    let connection = pool.get().await?;
    Ok(connection)
}

async fn add_msg_handler(
    pool: Pool,
    payload: &models::ClaimMintRewards,
    rate_limiter: &mut DirectRateLimiter<LeakyBucket>,
) -> Result<String, Rejection> {
    match rate_limiter.check() {
        Ok(_) => (),
        Err(_) => return Err(error::Error::RateLimitReachedError.into()),
    }

    let payload = serde_json::json!(payload).to_string();

    let rmq_con = get_rmq_con(pool.clone()).await.map_err(|e| {
        log::error!("can't connect to rmq, {}", e);
        warp::reject::custom(error::Error::RMQPoolError(e))
    })?;

    let channel = rmq_con.create_channel().await.map_err(|e| {
        log::error!("can't create channel, {}", e);
        warp::reject::custom(error::Error::RMQError(e))
    })?;

    let q = channel
        .queue_declare(
            "mint_response",
            lapin::options::QueueDeclareOptions::default(),
            lapin::types::FieldTable::default(),
        )
        .await
        .unwrap();
    log::debug!("Quelength: {}", q.message_count());

    channel
        .basic_publish(
            "",
            "mint_response",
            lapin::options::BasicPublishOptions::default(),
            payload.as_bytes(),
            lapin::BasicProperties::default(),
        )
        .await
        .map_err(|e| {
            log::error!("can't publish: {}", e);
            warp::reject::custom(error::Error::RMQError(e))
        })?
        .await
        .map_err(|e| {
            log::error!("can't publish: {}", e);
            warp::reject::custom(error::Error::RMQError(e))
        })?;
    Ok(serde_json::json!({"status":"successfull","spot":q.message_count()}).to_string())
}
