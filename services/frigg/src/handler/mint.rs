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
    /*
        let i = sleipnir::minting::api::import_nfts_from_csv_metadata(
            &hex::decode(params.csv_hex).unwrap(),
            user,
            params.project_id,
        )
        .await?;

        //Ok(warp::reply::with_status(
        //    warp::reply::json(&json!({ "imported": i })),
        //    warp::http::StatusCode::CREATED,
        //));
    */
    ////////////////////
    // let payload = serde_json::json!(payload).to_string();

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

    //let mut request_buf = String::new();
    //let mut file = File::create("/files/import_tmp").unwrap();
    //file.write_all(body.as_ref()).unwrap();
    //file.read_to_string(&mut request_buf).unwrap();
    /*
       let mut pinned_stream = Box::pin(body);
       while pinned_stream.has_remaining() {
           let r = pinned_stream.remaining();
           let b = pinned_stream.bytes();
           let mut i = 0;
           for (j, e) in b.enumerate() {
               if let Ok(byte) = e {
                   request_buf.push(byte)
               }
               i = j;
               log::debug!("{}", i);
           }
           if r > 0 {
               pinned_stream.advance(i);
           } else {
               break;
           }
           //request_buf.extend::<&[u8]>(body.as_ref());
       }
    */
    //let mut f = File::open(file).expect("no file found");
    //let metadata = file.metadata().expect("unable to read metadata");

    //let mut buffer = body.bytes();
    //file.read_to_end(&mut buffer).expect("buffer overflow");

    log::debug!("Request buffer: {:?}", &body);
    let i = drasil_sleipnir::minting::api::import_nfts_from_csv_metadata(body.as_ref(), user, mid)
        .await?;
    log::debug!("Debug: {:?}", &i);

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({ "imported": i })),
        warp::http::StatusCode::CREATED,
    ))
}
