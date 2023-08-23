mod model;
extern crate pretty_env_logger;

use drasil_murin::utxomngr::*;
use model::*;
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
struct StreamWorker {
    stream: String,
    consumer_group: String,
    worker_number: usize,
}

enum Connection {
    Cluster(redis::cluster::ClusterConnection),
    Single(redis::Connection),
}

fn connect() -> Result<Connection> {
    log::debug!("Try to connect to redis cluster...");

    let redis_cluster = std::env::var("REDIS_CLUSTER")?.parse::<bool>()?;

    if redis_cluster {
        let redis_db = std::env::var("REDIS_DB")?;
        let con = redis::cluster::ClusterClient::open(vec![redis_db])?.get_connection()?;

        Ok(Connection::Cluster(con))
    } else {
        let redis_db = std::env::var("REDIS_DB")?;
        let con = redis::Client::open(redis_db)?.get_connection()?;

        Ok(Connection::Single(con))
    }
}
pub fn run_worker(stream: String, worker_number: usize, id: String) -> Result<u8> {
    let con = connect()?;
    let worker = StreamWorker {
        stream: stream.clone(),
        consumer_group: stream.clone() + "_grp",
        worker_number,
    };
    log::info!("Worker redis request ...: {:?}", worker);

    #[allow(clippy::type_complexity)]
    let new_message: Vec<Vec<Vec<Vec<(String, Vec<String>)>>>> = match con {
        Connection::Cluster(mut c) => redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(worker.consumer_group.clone())
            .arg("worker_".to_string() + &worker.worker_number.to_string())
            .arg("COUNT")
            .arg("1")
            .arg("STREAMS")
            .arg(&worker.stream)
            .arg(id)
            .query(&mut c)?,
        Connection::Single(mut c) => redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(worker.consumer_group.clone())
            .arg("worker_".to_string() + &worker.worker_number.to_string())
            .arg("COUNT")
            .arg("1")
            .arg("STREAMS")
            .arg(&worker.stream)
            .arg(id)
            .query(&mut c)?,
    };

    log::info!(
        "New Message, Worker: {:?}, Stream: {:?}: {:?}",
        worker.worker_number,
        stream,
        new_message
    );

    if new_message.is_empty() {
        Ok(2)
    } else if new_message[0][0].is_empty() {
        Ok(1)
    } else {
        let data_vec = &new_message[0][0][0][0];
        let id = &data_vec.0;
        log::info!("ID:\n {:?} \n", id);
        let data = &data_vec.1;
        let event_type = data[0].clone();
        log::info!("Event Type: {}", event_type);
        let tx_data: Event = serde_json::from_str(&data[1])?;
        match tx_data.data {
            EventData::Transaction(tx) => {
                delete_used_utxo(&tx.hash)?;
                log::info!("Delete: {}", tx.hash);
            }
            _ => {
                log::info!("Event data is not a transaction");
            }
        }

        Ok(0)
    }
}

fn run_stream_trimmer(stream: String, maxlen: i32) -> Result<()> {
    let con = connect()?;
    match con {
        Connection::Cluster(mut c) => {
            redis::cmd("XTRIM")
                .arg(stream)
                .arg("MAXLEN")
                .arg("~")
                .arg(maxlen)
                .query(&mut c)?;
        }
        Connection::Single(mut c) => {
            redis::cmd("XTRIM")
                .arg(stream)
                .arg("MAXLEN")
                .arg("~")
                .arg(maxlen)
                .query(&mut c)?;
        }
    }
    Ok(())
}

//#[tokio::main]
pub fn main() -> Result<()> {
    use std::{thread, time};
    pretty_env_logger::init();

    let use_stream_trimmer = std::env::var("STREAM_TRIMMER")?.parse::<bool>()?;
    let streams = std::env::var("STREAMS")?;

    let slice: Vec<_> = streams.split('|').collect();
    let mut streams = Vec::<String>::new();
    streams.extend(slice.iter().map(|n| n.to_string()));

    log::debug!("Trying to establish first connection....");
    let timeout = std::env::var("TIMEOUT")?;
    let mut last_id: String;
    let mut read_backlog = true;

    let mut id: String;
    let mut init = true;

    let mut groups: Vec<Vec<String>>;
    for i in 0..streams.len() {
        log::info!("Try to get groups....");
        if init {
            let mcon = connect()?;
            match mcon {
                Connection::Cluster(mut c) => {
                    match redis::cmd("XINFO")
                        .arg("GROUPS")
                        .arg(streams[i].clone())
                        .query(&mut c)
                    {
                        Err(e) => {
                            log::info!("Got Err: {:?}", e.to_string());
                            let resp: String = redis::cmd("XGROUP")
                                .arg("CREATE")
                                .arg(streams[i].clone())
                                .arg(streams[i].clone() + "_grp")
                                .arg("$")
                                .query(&mut c)?;
                            groups = redis::cmd("XINFO")
                                .arg("GROUPS")
                                .arg(streams[i].clone())
                                .query(&mut c)?;
                            log::info!("Create Group: {:?}", resp);
                        }
                        Ok(ok) => {
                            log::info!("Got Ok: {:?}", ok);
                            groups = ok;
                        }
                    };
                    log::info!("Stream is: {:?}", streams.get(i));
                    log::info!("Checking groups....");
                    if groups.is_empty()
                        || !groups[0].contains(&(streams[i].clone() + "_grp").to_string())
                    {
                        let resp: String = redis::cmd("XGROUP")
                            .arg("CREATE")
                            .arg(streams[i].clone())
                            .arg(streams[i].clone() + "_grp")
                            .arg("$")
                            .query(&mut c)?;
                        log::info!("Create Group: {:?}", resp);
                    }
                }
                Connection::Single(mut c) => {
                    match redis::cmd("XINFO")
                        .arg("GROUPS")
                        .arg(streams[i].clone())
                        .query(&mut c)
                    {
                        // parse groups into data structure to get latest id
                        Err(e) => {
                            log::info!("Got Err: {:?}", e.to_string());
                            let resp: String = redis::cmd("XGROUP")
                                .arg("CREATE")
                                .arg(streams[i].clone())
                                .arg(streams[i].clone() + "_grp")
                                .arg("$")
                                .query(&mut c)?;
                            groups = redis::cmd("XINFO")
                                .arg("GROUPS")
                                .arg(streams[i].clone())
                                .query(&mut c)?;
                            log::info!("Create Group: {:?}", resp);
                        }
                        Ok(ok) => {
                            log::info!("Got Ok: {:?}", ok);
                            groups = ok;
                        }
                    };
                    log::info!("Stream is: {:?}", streams.get(i));
                    log::info!("Checking groups....");
                    if groups.is_empty()
                        || !groups[0].contains(&(streams[i].clone() + "_grp").to_string())
                    {
                        let resp: String = redis::cmd("XGROUP")
                            .arg("CREATE")
                            .arg(streams[i].clone())
                            .arg(streams[i].clone() + "_grp")
                            .arg("$")
                            .query(&mut c)?;
                        log::info!("Create Group: {:?}", resp);
                    }
                }
            }
        }
    }

    loop {
        let mut n = Vec::<u8>::new();

        log::info!("Worker loop....");
        for i in 0..streams.len() {
            match connect()? {
                Connection::Cluster(mut c) => {
                    groups = redis::cmd("XINFO")
                        .arg("GROUPS")
                        .arg(streams[i].clone())
                        .query(&mut c)?;
                }
                Connection::Single(mut c) => {
                    groups = redis::cmd("XINFO")
                        .arg("GROUPS")
                        .arg(streams[i].clone())
                        .query(&mut c)?;
                }
            }
            last_id = groups[0][5].clone();
            if read_backlog {
                id = last_id.clone();
            } else {
                id = ">".to_string();
            }
            log::info!("Last ID: {:?}", last_id);
            let stream = streams[i].clone();
            log::info!("Run worker for stream: {:?}....", stream);
            n.push(match run_worker(stream, 0, id.clone()) {
                Ok(n) => n,
                Err(e) => {
                    log::error!("failed to start stream-worker, '{:?}'", e);
                    0
                }
            });

            let min_value = n.iter().min();
            let n = match min_value {
                Some(min) => *min,
                None => 2u8,
            };

            if n >= 2 {
                let start = std::time::Instant::now();
                if use_stream_trimmer {
                    for stream in streams.clone() {
                        let maxlen = match &stream.clone()[..] {
                            "transaction" => 25000,
                            "block" => 5000,
                            "rollback" => 1000,
                            _ => 1000,
                        };
                        run_stream_trimmer(stream, maxlen)?;
                        log::info!("Ran stream trimmer")
                    }
                }
                let duration = start.elapsed();

                let millis = time::Duration::from_millis(timeout.parse::<u64>()?);
                log::info!("--------------------------------------------------------------------------------------------");
                thread::sleep(millis - duration);
            }
            if n == 1 {
                read_backlog = false;
            }
        }

        if init {
            init = false;
        }
    }
}
