/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::{
    redis_usedutxos_connection, MurinError, TransactionUnspentOutput, TransactionUnspentOutputs,
};
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsedUtxo {
    txhash: String,
    index: u32,
}

#[derive(Debug, Clone)]
pub struct TxUsedUtxos {
    txhash: String,
    utxos: Vec<UsedUtxo>,
}

impl TxUsedUtxos {
    pub fn new(txhash: &str, txuo: &TransactionUnspentOutputs) -> TxUsedUtxos {
        let mut used_utxos = Vec::<UsedUtxo>::new();
        for i in 0..txuo.len() {
            used_utxos.push(UsedUtxo {
                txhash: hex::encode(txuo.get(i).input().transaction_id().to_bytes()),
                index: txuo.get(i).input().index(),
            })
        }

        TxUsedUtxos {
            txhash: txhash.to_owned(),
            utxos: used_utxos,
        }
    }

    pub fn get_txhash(&self) -> &String {
        &self.txhash
    }

    pub fn get_used_utxos(&self) -> &Vec<UsedUtxo> {
        &self.utxos
    }
}

impl UsedUtxo {
    pub fn get_txhash(&self) -> &String {
        &self.txhash
    }

    pub fn get_index(&self) -> u32 {
        self.index
    }
}

impl ToString for UsedUtxo {
    fn to_string(&self) -> String {
        self.txhash.clone() + "#" + &self.index.to_string()
    }
}

pub fn utxovec_to_utxostring(utxos: &Vec<String>) -> String {
    let mut str = String::new();
    for u in utxos {
        str.push_str(u);
        str.push('|');
    }
    str.pop();
    str
}

pub fn utxostring_to_utxovec(str: &str) -> Vec<String> {
    let slice: Vec<&str> = str.split('|').collect();
    let mut vec = Vec::<String>::new();
    for s in slice {
        vec.push(s.to_string())
    }
    vec
}

pub fn store_used_utxos(
    txhash: &String,
    txuos: &TransactionUnspentOutputs,
) -> Result<(), MurinError> {
    let con = redis_usedutxos_connection()?;
    info!("storing used utxos...");
    let used = TxUsedUtxos::new(txhash, txuos);
    let len = used.get_used_utxos().len();
    let key = select_used_utxo_datastore(len, None)?;
    let mut payload = Vec::<String>::new();
    for utxo in used.get_used_utxos() {
        payload.push(utxo.to_string())
    }
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let items: [(&str, &str); 2] = [
        ("utxos", &utxovec_to_utxostring(&payload)),
        ("timestamp", &timestamp),
    ];
    match con {
        (Some(mut c), None) => {
            redis::cmd("SADD").arg(&key.0).arg(&payload).query(&mut c)?;
            redis::cmd("HMSET").arg(txhash).arg(&items).query(&mut c)?;
        }
        (None, Some(mut c)) => {
            redis::cmd("SADD").arg(&key.0).arg(&payload).query(&mut c)?;
            redis::cmd("HMSET").arg(txhash).arg(&items).query(&mut c)?;
        }
        _ => {}
    }
    debug!("Stored Used Utxos...: {:?}", txuos);
    Ok(())
}

pub fn store_used_utxos_from_txm(txhash: &String, utxo: &Vec<String>) -> Result<(), MurinError> {
    let mut con = redis_usedutxos_connection()?;

    let latesttx = "txlatest".to_string();
    let mut response: i64 = 0;
    match con {
        (Some(ref mut c), None) => {
            response = redis::cmd("SISMEMBER")
                .arg(&latesttx)
                .arg(txhash)
                .query(c)?;
        }
        (None, Some(ref mut c)) => {
            response = redis::cmd("SISMEMBER")
                .arg(&latesttx)
                .arg(txhash)
                .query(c)?;
        }
        _ => {}
    }

    if response == 0 {
        let txgmempool = std::env::var("TXGSET")?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let _is_hex = hex::decode(txhash)?;
        for u in utxo {
            let split: Vec<_> = u.split('#').collect();
            hex::decode(split[0])?;
        }

        info!("storing used utxos...");
        let len = utxo.len();
        let key = select_used_utxo_datastore(len, None)?;

        let mut payload = Vec::<String>::new();
        for u in utxo {
            payload.push(u.clone())
        }
        let items: [(&str, &str); 2] = [
            ("utxos", &utxovec_to_utxostring(&payload)),
            ("timestamp", &timestamp),
        ];
        match con {
            (Some(ref mut c), None) => {
                redis::cmd("SADD").arg(&key.0).arg(&payload).query(c)?;
                redis::cmd("HMSET").arg(txhash).arg(&items).query(c)?;
                //redis::cmd("EXPIRE").arg(&txhash).arg("1800").arg("NX").query(c)?;
                redis::cmd("SADD").arg(&txgmempool).arg(txhash).query(c)?;
                redis::cmd("SADD").arg(&latesttx).arg(txhash).query(c)?;
            }
            (None, Some(ref mut c)) => {
                redis::cmd("SADD").arg(&key.0).arg(&payload).query(c)?;
                redis::cmd("HMSET").arg(txhash).arg(&items).query(c)?;
                //redis::cmd("EXPIRE").arg(&txhash).arg("1800").arg("NX").query(c)?;
                redis::cmd("SADD").arg(&txgmempool).arg(txhash).query(c)?;
                redis::cmd("SADD").arg(&latesttx).arg(txhash).query(c)?;
            }
            _ => {}
        }
    }

    debug!("Stored Used Utxos...: {:?}", utxo);
    Ok(())
}

pub fn store_used_utxos_from_txm_org(
    txhash: &String,
    utxo: &Vec<String>,
) -> Result<(), MurinError> {
    let mut con = redis_usedutxos_connection()?;

    let txgmempool = std::env::var("TXGSET")?;

    let _is_hex = hex::decode(txhash)?;
    for u in utxo {
        let split: Vec<_> = u.split('#').collect();
        hex::decode(split[0])?;
    }

    info!("storing used utxos...");
    let len = utxo.len();
    let key = select_used_utxo_datastore(len, None)?;

    let mut payload = Vec::<String>::new();
    for u in utxo {
        payload.push(u.clone())
    }

    match con {
        (Some(ref mut c), None) => {
            redis::cmd("SADD").arg(&key.0).arg(&payload).query(c)?;
            redis::cmd("SADD").arg(txhash).arg(&payload).query(c)?;
            redis::cmd("EXPIRE")
                .arg(txhash)
                .arg("3600")
                .arg("NX")
                .query(c)?;
            redis::cmd("SADD").arg(&txgmempool).arg(txhash).query(c)?;
        }
        (None, Some(ref mut c)) => {
            redis::cmd("SADD").arg(&key.0).arg(&payload).query(c)?;
            redis::cmd("SADD").arg(txhash).arg(&payload).query(c)?;
            redis::cmd("EXPIRE")
                .arg(txhash)
                .arg("3600")
                .arg("NX")
                .query(c)?;
            redis::cmd("SADD").arg(&txgmempool).arg(txhash).query(c)?;
        }
        _ => {}
    }

    debug!("Stored Used Utxos...: {:?}", utxo);
    Ok(())
}

/// return all pending UTxOs
pub fn get_all_pending_utxos() -> Result<Vec<String>, MurinError> {
    let mut con = redis_usedutxos_connection()?;
    let mut out = Vec::<String>::new();
    for i in 0..2 {
        let key = select_used_utxo_datastore(0, Some(i))?;
        match con {
            (Some(ref mut c), None) => {
                let response: Vec<String> = redis::cmd("SMEMBERS").arg(key.0).query(c)?;
                out.extend(response.iter().map(|n| n.to_owned()));
            }
            (None, Some(ref mut c)) => {
                let response: Vec<String> = redis::cmd("SMEMBERS").arg(key.0).query(c)?;
                out.extend(response.iter().map(|n| n.to_owned()));
            }
            _ => {}
        };
    }

    Ok(out)
}

/// return all pending transactions
pub fn get_all_pending_tx() -> Result<Vec<String>, MurinError> {
    let mut con = redis_usedutxos_connection()?;
    let txgmempool = std::env::var("TXGSET").expect("TXGSET not set");
    let mut out = Vec::<String>::new();
    match con {
        (Some(ref mut c), None) => {
            let response: Vec<String> = redis::cmd("SMEMBERS").arg(txgmempool).query(c)?;
            out.extend(response.iter().map(|n| n.to_owned()));
        }
        (None, Some(ref mut c)) => {
            let response: Vec<String> = redis::cmd("SMEMBERS").arg(txgmempool).query(c)?;
            out.extend(response.iter().map(|n| n.to_owned()));
        }
        _ => {}
    };
    Ok(out)
}

/// checks if a single utxo is used already
pub fn check_utxo_used(txuo: &TransactionUnspentOutput) -> Result<bool, MurinError> {
    let mut con = redis_usedutxos_connection()?;
    let member = hex::encode(txuo.input().transaction_id().to_bytes())
        + "#"
        + &txuo.input().index().to_string();
    let mut response: i64 = 0;
    for i in 0..2 {
        let key = select_used_utxo_datastore(0, Some(i))?;
        match con {
            (Some(ref mut c), None) => {
                response = redis::cmd("SISMEMBER").arg(key.0).arg(&member).query(c)?;
            }
            (None, Some(ref mut c)) => {
                response = redis::cmd("SISMEMBER").arg(key.0).arg(&member).query(c)?;
            }
            _ => {
                response = -1;
            }
        };
    }
    if response > 0 {
        return Ok(true);
    }
    Ok(false)
}

fn sismember(
    con: &mut (
        Option<redis::cluster::ClusterConnection>,
        Option<redis::Connection>,
    ),
    key: &String,
    utxos: &Vec<String>,
) -> Vec<i64> {
    match con {
        (Some(ref mut c), None) => match redis::cmd("SMISMEMBER").arg(key).arg(utxos).query(c) {
            Ok(o) => o,
            Err(e) => {
                log::error!("Could not find used utxo members, error: {}", e.to_string());
                vec![]
            }
        },
        (None, Some(ref mut c)) => match redis::cmd("SMISMEMBER").arg(key).arg(utxos).query(c) {
            Ok(o) => o,
            Err(e) => {
                log::error!("Could not find used utxo members, error: {}", e.to_string());
                vec![]
            }
        },
        _ => {
            vec![]
        }
    }
}
/// Return a list of valid utxos from a given utxos list
pub fn get_valid_utxos_sif(utxos_in: &[String]) -> Result<Vec<String>, MurinError> {
    info!("check used utxos...");
    let mut con = redis_usedutxos_connection()?;
    let mut utxos = utxos_in.to_owned();

    for i in &[0, 1, 2] {
        let key = select_used_utxo_datastore(0, Some(*i))?;
        if !key.0.is_empty() && key.1 > 0 {
            debug!("Key: {:?}", key);
            let response = sismember(&mut con, &key.0, &utxos);
            debug!("\n\nResponse: {:?}", response);
            for (j, i) in response.into_iter().enumerate() {
                if i > 0 {
                    utxos.remove(j);
                }
            }
        }
    }
    Ok(utxos)
}

pub fn check_any_utxo_used(
    txuos: &TransactionUnspentOutputs,
) -> Result<Option<Vec<UsedUtxo>>, MurinError> {
    info!("check used utxos...");
    let con = redis_usedutxos_connection();

    let mut con = match con {
        Ok(o) => {
            log::debug!("Redis connection ok");
            o
        }
        Err(e) => {
            log::debug!("Redis connection failed with: {:?}", e.to_string());
            return Err(MurinError::new(&e.to_string()));
        }
    };
    let mut members = Vec::<String>::new();
    debug!("Input UTxOs: '{:?}'", txuos);
    for j in 0..txuos.len() {
        members.push(
            hex::encode(txuos.get(j).input().transaction_id().to_bytes())
                + "#"
                + &txuos.get(j).input().index().to_string(),
        );
    }
    debug!("\n\nMembers: {:?}", members);
    let mut used_utxos = Vec::<UsedUtxo>::new();

    for i in &[0, 1, 2] {
        let key = select_used_utxo_datastore(0, Some(*i))?;
        if !key.0.is_empty() && key.1 > 0 {
            //debug!("Key: {:?}", key);
            let response = sismember(&mut con, &key.0, &members);
            debug!("\n\nResponse: {:?}", response);
            for (j, i) in response.into_iter().enumerate() {
                if i > 0 {
                    let u: Vec<&str> = members.get(j).unwrap().split('#').collect();
                    used_utxos.push(UsedUtxo {
                        txhash: u[0].to_string(),
                        index: u[1].parse::<u32>()?,
                    })
                }
            }
        }
    }
    if !used_utxos.is_empty() {
        Ok(Some(used_utxos))
    } else {
        Ok(None)
    }
}

/// deletes a used utxo
pub fn delete_used_utxo(txhash: &String) -> Result<(), MurinError> {
    let mut con = redis_usedutxos_connection()?;
    info!("deleting pending utxo...");
    info!("TxHash: {:?}", txhash);
    let mut members = Vec::<String>::new();
    match con {
        (Some(ref mut c), None) => {
            let tx_map = redis::cmd("HGETALL").arg(txhash).query(c);
            debug!("Tx Hashmap Result: {:?}", tx_map);
            let tx_map: HashMap<String, String> = tx_map?;
            if tx_map.contains_key("utxos") {
                members = utxostring_to_utxovec(
                    tx_map
                        .get("utxos")
                        .expect("Could not retrieve utxos from hashmap"),
                );
            }
        }
        (None, Some(ref mut c)) => {
            let tx_map = redis::cmd("HGETALL").arg(txhash).query(c);
            debug!("Tx Hashmap Result: {:?}", tx_map);
            let tx_map: HashMap<String, String> = tx_map?;
            if tx_map.contains_key("utxos") {
                members = utxostring_to_utxovec(
                    tx_map
                        .get("utxos")
                        .expect("Could not retrieve utxos from hashmap"),
                );
            }
        }
        _ => {
            return Err(MurinError::new(
                "Could not establish single nor cluster redis connection",
            ));
        }
    };

    info!("Members to be deleted: {:?}", members);
    if !members.is_empty() {
        for i in 0..2 {
            let key = select_used_utxo_datastore(0, Some(i))?;
            match con {
                (Some(ref mut c), None) => {
                    redis::cmd("SREM").arg(&key.0).arg(&members).query(c)?;
                }
                (None, Some(ref mut c)) => {
                    redis::cmd("SREM").arg(&key.0).arg(&members).query(c)?;
                }
                _ => {
                    return Err(MurinError::new(
                        "Could not establish single nor cluster redis connection",
                    ));
                }
            };
        }
        match con {
            (Some(ref mut c), None) => {
                redis::cmd("SREM").arg("txgmempool").arg(txhash).query(c)?;
                redis::cmd("DEL").arg(txhash).query(c)?;
            }
            (None, Some(ref mut c)) => {
                redis::cmd("SREM").arg("txgmempool").arg(txhash).query(c)?;
                redis::cmd("DEL").arg(txhash).query(c)?;
            }
            _ => {
                return Err(MurinError::new(
                    "Could not establish single nor cluster redis connection",
                ));
            }
        };
    }

    Ok(())
}

/// deletes a used utxo
pub async fn delete_used_utxo_async(
    redis_con: &mut redis::aio::Connection,
    txhash: &String,
) -> Result<(), MurinError> {
    info!("deleting pending utxo...");
    info!("TxHash: {:?}", txhash);
    let tx_map = redis::cmd("HGETALL")
        .arg(txhash)
        .query_async(redis_con)
        .await;
    debug!("Tx Hashmap Result: {:?}", tx_map);
    let tx_map: HashMap<String, String> = tx_map?;
    let members = utxostring_to_utxovec(
        tx_map
            .get("utxos")
            .expect("Could not retrieve utxos from hashmap"),
    );
    info!("Members: {:?}", members);
    if !members.is_empty() {
        for i in 0..2 {
            let key = select_used_utxo_datastore(0, Some(i))?;
            redis::cmd("SREM")
                .arg(&key.0)
                .arg(&members)
                .query_async(redis_con)
                .await?;
        }
    }
    redis::cmd("SREM")
        .arg("txgmempool")
        .arg(txhash)
        .query_async(redis_con)
        .await?;
    redis::cmd("DEL").arg(txhash).query_async(redis_con).await?;
    info!("Delete Response: {:?}", txhash);
    Ok(())
}

/// deletes a used utxo
pub async fn delete_used_utxos_hashmap_async(
    redis_con: &mut redis::aio::Connection,
    txhash: &String,
    members: &Vec<String>,
) -> Result<(), MurinError> {
    info!("deleting pending utxo...");
    debug!("TxHash: {:?}", txhash);
    debug!("Members: {:?}", members);
    for i in 0..2 {
        let key = select_used_utxo_datastore(0, Some(i))?;
        redis::cmd("SREM")
            .arg(&key.0)
            .arg(members)
            .query_async(redis_con)
            .await?;
    }
    redis::cmd("SREM")
        .arg("txgmempool")
        .arg(txhash)
        .query_async(redis_con)
        .await?;
    redis::cmd("DEL").arg(txhash).query_async(redis_con).await?;
    info!("Delete Response: {:?}", txhash);
    Ok(())
}

/// Select a data store for used utxos, selects first one if enough free space otherwise tries 2 and 3
fn select_used_utxo_datastore(len: usize, get_ds: Option<u8>) -> Result<(String, i64), MurinError> {
    info!("Select datastore...");
    let datastores = vec![
        std::env::var("USED_UTXO_DATASTORE_1").unwrap_or_else(|_| "usedutxos1".to_string()),
        std::env::var("USED_UTXO_DATASTORE_2").unwrap_or_else(|_| "usedutxos2".to_string()),
        std::env::var("USED_UTXO_DATASTORE_3").unwrap_or_else(|_| "usedutxos3".to_string()),
    ];

    let mut con = redis_usedutxos_connection()?;
    if let Some(i) = get_ds {
        let ds_card: i64 = match con {
            (Some(ref mut c), None) => redis::cmd("SCARD").arg(&datastores[i as usize]).query(c)?,
            (None, Some(ref mut c)) => redis::cmd("SCARD").arg(&datastores[i as usize]).query(c)?,
            _ => {
                return Err(MurinError::new(
                    "Could not establish single nor cluster redis connection",
                ));
            }
        };

        return Ok((datastores[i as usize].clone(), ds_card));
    }

    for ds in datastores {
        let ds_card: i64 = match con {
            (Some(ref mut c), None) => redis::cmd("SCARD").arg(&ds).query(c)?,
            (None, Some(ref mut c)) => redis::cmd("SCARD").arg(&ds).query(c)?,
            _ => {
                return Err(MurinError::new(
                    "Could not establish single nor cluster redis connection",
                ));
            }
        };
        if ds_card < 4294967295 - (len as i64) {
            return Ok((ds, ds_card));
        }
    }
    Err(MurinError::new(
        "No available space in datastores, are enough 'used_utxo' datastores set?",
    ))
}

/// Select a datastore for pending transactions (not needed)
fn _select_pending_tx_datastore(
    len: usize,
    get_ds: Option<u8>,
) -> Result<(String, i64), MurinError> {
    let datastores = vec![
        std::env::var("PENDING_TX_DATASTORE_1").unwrap_or_else(|_| "pendingtx1".to_string()),
        std::env::var("PENDING_TX_DATASTORE_2").unwrap_or_else(|_| "pendingtx2".to_string()),
        std::env::var("PENDING_TX_DATASTORE_3").unwrap_or_else(|_| "pendingtx3".to_string()),
    ];

    let mut con = redis_usedutxos_connection()?;
    if let Some(i) = get_ds {
        let ds_card: i64 = match con {
            (Some(ref mut c), None) => redis::cmd("SCARD").arg(&datastores[i as usize]).query(c)?,
            (None, Some(ref mut c)) => redis::cmd("SCARD").arg(&datastores[i as usize]).query(c)?,
            _ => {
                return Err(MurinError::new(
                    "Could not establish single nor cluster redis connection",
                ));
            }
        };
        return Ok((datastores[i as usize].clone(), ds_card));
    }

    for ds in datastores {
        let ds_card: i64 = match con {
            (Some(ref mut c), None) => redis::cmd("SCARD").arg(&ds).query(c)?,
            (None, Some(ref mut c)) => redis::cmd("SCARD").arg(&ds).query(c)?,
            _ => {
                return Err(MurinError::new(
                    "Could not establish single nor cluster redis connection",
                ));
            }
        };
        if ds_card < 4294967295 - (len as i64) {
            return Ok((ds, ds_card));
        }
    }
    Err(MurinError::new(
        "No available space in datastores, are enough 'used_utxo' datastores set?",
    ))
}
