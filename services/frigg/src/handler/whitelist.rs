use deadpool_lapin::Pool;
use drasil_sleipnir::whitelist::{
    AllocateSpecificAssetsToMintProject, ImportWhitelistFromCSV, WlNew,
};
use serde::{Deserialize, Serialize};
use warp::Reply;

use crate::{
    handler::{get_rmq_con, get_user_from_string, JOB_QUEUE_NAME},
    WebResult,
};
#[derive(Serialize, Deserialize, Debug)]
pub struct WlId {
    whitelist_id: i64,
}

pub async fn create_whitelist(uid: String, params: WlNew) -> WebResult<impl Reply> {
    log::debug!("create_whitelist");
    let user = get_user_from_string(&uid).await?;
    log::debug!("Data: {:?}", params);
    let result = drasil_sleipnir::whitelist::create_whitelist(&user, &vec![params])?;

    Ok(serde_json::json!(result).to_string())
}

pub async fn delete_whitelist(uid: String, params: WlId) -> WebResult<impl Reply> {
    log::debug!("delete_whitelist");
    let user = get_user_from_string(&uid).await?;
    log::debug!("Data: {:?}", params);
    let result = drasil_sleipnir::whitelist::delete_whitelists(&user, &params.whitelist_id)?;
    Ok(serde_json::json!(result).to_string())
}

pub async fn import_whitelist_from_csv(
    uid: String,
    pool: Pool,
    params: ImportWhitelistFromCSV,
) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;
    let job = drasil_sleipnir::jobs::Job {
        drasil_user_id: user,
        session_id: None,
        data: serde_json::json!(params),
    };

    let job = drasil_sleipnir::jobs::JobTypes::ImportWhitelist(job);

    let rmq_con = get_rmq_con(pool.clone()).await.map_err(|e| {
        log::error!("can't connect to rmq, {}", e);
        warp::reject::custom(crate::error::Error::RMQPoolError(e))
    })?;

    let channel = rmq_con.create_channel().await.map_err(|e| {
        log::error!("can't create channel, {}", e);
        warp::reject::custom(crate::error::Error::RMQError(e))
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
            warp::reject::custom(crate::error::Error::RMQError(e))
        })?
        .await
        .map_err(|e| {
            log::error!("can't publish: {}", e);
            warp::reject::custom(crate::error::Error::RMQError(e))
        })?;
    Ok(
        serde_json::json!({"status":"import queued","queue position":q.message_count()})
            .to_string(),
    )
}

pub async fn allocate_whitelist_to_mp(
    uid: String,
    pool: Pool,
    params: AllocateSpecificAssetsToMintProject,
) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;
    let job = drasil_sleipnir::jobs::Job {
        drasil_user_id: user,
        session_id: None,
        data: serde_json::json!(params),
    };

    let job = drasil_sleipnir::jobs::JobTypes::AllocateSpecificAssetsToMintProject(job);

    let rmq_con = get_rmq_con(pool.clone()).await.map_err(|e| {
        log::error!("can't connect to rmq, {}", e);
        warp::reject::custom(crate::error::Error::RMQPoolError(e))
    })?;

    let channel = rmq_con.create_channel().await.map_err(|e| {
        log::error!("can't create channel, {}", e);
        warp::reject::custom(crate::error::Error::RMQError(e))
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
            warp::reject::custom(crate::error::Error::RMQError(e))
        })?
        .await
        .map_err(|e| {
            log::error!("can't publish: {}", e);
            warp::reject::custom(crate::error::Error::RMQError(e))
        })?;
    Ok(
        serde_json::json!({"status":"import queued","queue position":q.message_count()})
            .to_string(),
    )
}

pub async fn random_allocate_whitelist_to_mp(
    uid: String,
    pool: Pool,
    params: AllocateSpecificAssetsToMintProject,
) -> WebResult<impl Reply> {
    let user = get_user_from_string(&uid).await?;
    let job = drasil_sleipnir::jobs::Job {
        drasil_user_id: user,
        session_id: None,
        data: serde_json::json!(params),
    };

    let job = drasil_sleipnir::jobs::JobTypes::RandomAllocateWhitelistToMintProject(job);

    let rmq_con = get_rmq_con(pool.clone()).await.map_err(|e| {
        log::error!("can't connect to rmq, {}", e);
        warp::reject::custom(crate::error::Error::RMQPoolError(e))
    })?;

    let channel = rmq_con.create_channel().await.map_err(|e| {
        log::error!("can't create channel, {}", e);
        warp::reject::custom(crate::error::Error::RMQError(e))
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
            warp::reject::custom(crate::error::Error::RMQError(e))
        })?
        .await
        .map_err(|e| {
            log::error!("can't publish: {}", e);
            warp::reject::custom(crate::error::Error::RMQError(e))
        })?;
    Ok(
        serde_json::json!({"status":"import queued","queue position":q.message_count()})
            .to_string(),
    )
}
