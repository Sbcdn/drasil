use super::get_user_from_string;
use crate::{
    error,
    handler::{get_rmq_con, JOB_QUEUE_NAME},
    WebResult,
};
use deadpool_lapin::Pool;
use drasil_sleipnir::models::{CreateMintProj, ImportNFTsfromCSV};
use serde_json::json;
use warp::Reply;

pub async fn entrp_create_mint_proj(uid: String, param: CreateMintProj) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;
    let mut param = param.clone();
    param.user_id = Some(user);

    let contract_id = drasil_sleipnir::minting::api::create_mintproject(&param).await?;
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "contract_id": contract_id })),
        warp::http::StatusCode::CREATED,
    ))
}

pub async fn entrp_create_nfts_from_csv(
    uid: String,
    pool: Pool,
    params: ImportNFTsfromCSV,
) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    let job = drasil_sleipnir::jobs::Job {
        drasil_user_id: user,
        session_id: None,
        data: serde_json::json!(params),
    };

    let job = drasil_sleipnir::jobs::JobTypes::ImportNFTsFromCsv(job);

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
            JOB_QUEUE_NAME.as_str(),
            lapin::options::QueueDeclareOptions::default(),
            lapin::types::FieldTable::default(),
        )
        .await
        .unwrap();
    log::debug!("Quelength: {}", q.message_count());

    channel
        .basic_publish(
            "",
            JOB_QUEUE_NAME.as_str(),
            lapin::options::BasicPublishOptions::default(),
            serde_json::to_string(&job)
                .map_err(|_| crate::error::Error::Custom("serde serialization failed".to_owned()))?
                .as_bytes(),
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
    Ok(
        serde_json::json!({"status":"import queued","queue position":q.message_count()})
            .to_string(),
    )
}

pub async fn entrp_create_nfts_from_csv_s(
    uid: String,
    mid: i64,
    body: bytes::Bytes,
) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;

    log::debug!("Request buffer: {:?}", &body);
    let i = drasil_sleipnir::minting::api::import_nfts_from_csv_metadata(body.as_ref(), user, mid)
        .await?;
    log::debug!("Debug: {:?}", &i);

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "imported": i })),
        warp::http::StatusCode::CREATED,
    ))
}
