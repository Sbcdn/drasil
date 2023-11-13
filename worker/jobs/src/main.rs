extern crate pretty_env_logger;

mod error;
mod handlers;
mod models;

use deadpool_lapin::Pool;
use lapin::ConnectionProperties;
use lazy_static::lazy_static;
use rand::prelude::*;
use std::env;

use drasil_sleipnir::jobs::JobTypes;

lazy_static! {
    static ref AMQP_ADDR: String =
        std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rmq:rmq@127.0.0.1:5672/%2f".into());
    static ref JOB_QUEUE_NAME: String =
        std::env::var("JOB_QUEUE_NAME").unwrap_or_else(|_| "drasil_jobs".to_string());
    static ref CONSUMER_NAME: String =
        std::env::var("CONSUMER_NAME").unwrap_or_else(|_| "job_processor_".to_string());
}

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    // RMQ
    let manager =
        deadpool_lapin::Manager::new(AMQP_ADDR.to_string(), ConnectionProperties::default());
    let pool: deadpool_lapin::Pool = deadpool::managed::Pool::builder(manager)
        .max_size(100)
        .build()
        .expect("can't create pool");
    log::debug!("pool: {:?}", pool);

    let _ = futures::join!(rmq_listen(pool));
    Ok(())
}

async fn get_rmq_con(pool: Pool) -> Result<models::Connection, deadpool_lapin::PoolError> {
    let connection = pool.get().await?;
    Ok(connection)
}

async fn rmq_listen(pool: deadpool_lapin::Pool) -> Result<(), error::Error> {
    let mut retry_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    loop {
        retry_interval.tick().await;
        log::debug!("connecting rmq consumer...");
        match init_rmq_listen(pool.clone()).await {
            Ok(_) => log::debug!("rmq listen returned"),
            Err(e) => log::debug!("rmq listen had an error: {}", e),
        };
    }
}

async fn init_rmq_listen(pool: Pool) -> Result<(), error::Error> {
    let rmq_con = get_rmq_con(pool).await.map_err(|e| {
        log::error!("could not get rmq con: {}", e);
        e
    })?;
    let channel = rmq_con.create_channel().await?;

    let queue = channel
        .queue_declare(
            &JOB_QUEUE_NAME.to_string(),
            lapin::options::QueueDeclareOptions::default(),
            lapin::types::FieldTable::default(),
        )
        .await?;
    log::debug!("Declared queue {:?}", queue);

    let mut consumer = channel
        .basic_consume(
            &JOB_QUEUE_NAME.to_string(),
            &(CONSUMER_NAME.to_owned() + &random::<u64>().to_string()),
            lapin::options::BasicConsumeOptions::default(),
            lapin::types::FieldTable::default(),
        )
        .await?;

    log::debug!("rmq consumer connected, waiting for messages");
    while let Some(delivery) = tokio_stream::StreamExt::next(&mut consumer).await {
        if let Ok(deliv) = delivery {
            match handlers::handle_job(&serde_json::from_slice::<JobTypes>(&deliv.data)?).await {
                Ok(_) => {
                    log::info!("Successfull");
                    channel
                        .basic_ack(
                            deliv.delivery_tag,
                            lapin::options::BasicAckOptions::default(),
                        )
                        .await?
                }
                Err(e) => {
                    log::error!("{:?}", e.to_string());
                    channel
                        .basic_reject(
                            deliv.delivery_tag,
                            lapin::options::BasicRejectOptions::default(),
                        )
                        .await?
                }
            }
        }
    }
    Ok(())
}
