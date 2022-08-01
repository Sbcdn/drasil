/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::utxomngr::*;
use crate::MurinError;
use sha2::Digest;
use std::collections::HashMap;
pub struct TxMindId {
    pub id: String,
}

impl TxMindId {
    fn new(raw: &RawTx) -> TxMindId {
        let mut hasher = sha2::Sha224::new();
        hasher.update(raw.tx_unsigned.as_bytes());
        hasher.update(raw.tx_aux.as_bytes());
        hasher.update(raw.stake_addr.as_bytes());
        hasher.update(raw.tx_raw_data.as_bytes());
        hasher.update(raw.specific_raw_data.as_bytes());
        let result = hasher.finalize();

        TxMindId {
            id: hex::encode(result),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawTx {
    tx_body: String,
    tx_witness: String,
    tx_unsigned: String,
    tx_aux: String,
    tx_raw_data: String,
    specific_raw_data: String,
    used_utxos: String,
    stake_addr: String,
    user_id: String,
    contract_id: String,
    contract_version: String,
}

impl RawTx {
    pub fn new_empty() -> RawTx {
        RawTx {
            tx_body: "".to_string(),
            tx_witness: "".to_string(),
            tx_unsigned: "".to_string(),
            tx_aux: "".to_string(),
            tx_raw_data: "".to_string(),
            specific_raw_data: "".to_string(),
            used_utxos: "".to_string(),
            stake_addr: "".to_string(),
            user_id: "".to_string(),
            contract_id: "".to_string(),
            contract_version: "".to_string(),
        }
    }

    pub fn new(
        tx_body: &String,
        tx_witness: &String,
        tx_unsigned: &String,
        tx_aux: &String,
        tx_raw_data: &String,
        specific_raw_data: &String,
        used_utxos: &String,
        stake_addr: &String,
        user_id: &i64,
        contract_id: &i64,
        contract_version: &f32,
    ) -> RawTx {
        RawTx {
            tx_body: tx_body.to_string(),
            tx_witness: tx_witness.to_string(),
            tx_unsigned: tx_unsigned.to_string(),
            tx_aux: tx_aux.to_string(),
            tx_raw_data: tx_raw_data.to_string(),
            specific_raw_data: specific_raw_data.to_string(),
            used_utxos: used_utxos.to_string(),
            stake_addr: stake_addr.to_string(),
            user_id: user_id.to_string(),
            contract_id: contract_id.to_string(),
            contract_version: contract_version.to_string(),
        }
    }

    pub fn get_txbody(&self) -> &String {
        &self.tx_body
    }

    pub fn get_txwitness(&self) -> &String {
        &self.tx_witness
    }

    pub fn get_txunsigned(&self) -> &String {
        &self.tx_unsigned
    }

    pub fn get_txaux(&self) -> &String {
        &self.tx_aux
    }

    pub fn get_txrawdata(&self) -> &String {
        &self.tx_raw_data
    }

    pub fn get_tx_specific_rawdata(&self) -> &String {
        &self.specific_raw_data
    }

    pub fn get_usedutxos(&self) -> &String {
        &self.used_utxos
    }

    pub fn get_stake_addr(&self) -> &String {
        &self.stake_addr
    }
    pub fn get_user_id(&self) -> Result<i64, MurinError> {
        Ok(self.user_id.parse::<i64>()?)
    }

    pub fn get_user_id_as_str(&self) -> &String {
        &self.user_id
    }
    pub fn get_contract_id(&self) -> Result<i64, MurinError> {
        Ok(self.contract_id.parse::<i64>()?)
    }

    pub fn get_contract_id_as_str(&self) -> &String {
        &self.contract_id
    }

    pub fn get_contract_version(&self) -> Result<f32, MurinError> {
        Ok(self.contract_version.parse::<f32>()?)
    }

    pub fn get_contract_version_as_str(&self) -> &String {
        &self.contract_version
    }

    pub fn set_txbody(&mut self, str: &String) -> () {
        self.tx_body = str.clone();
    }

    pub fn set_txwitness(&mut self, str: &String) -> () {
        self.tx_witness = str.clone();
    }

    pub fn set_txunsigned(&mut self, str: &String) -> () {
        self.tx_unsigned = str.clone();
    }

    pub fn set_txaux(&mut self, str: &String) -> () {
        self.tx_aux = str.clone();
    }

    pub fn set_txrawdata(&mut self, str: &String) -> () {
        self.tx_raw_data = str.clone();
    }

    pub fn set_tx_specific_rawdata(&mut self, str: &String) -> () {
        self.specific_raw_data = str.clone();
    }

    pub fn set_usedutxos(&mut self, str: &String) -> () {
        self.used_utxos = str.clone();
    }

    pub fn set_stake_addr(&mut self, str: &String) -> () {
        self.stake_addr = str.clone();
    }

    pub fn set_contract_id(&mut self, n: i64) -> () {
        self.contract_id = n.to_string();
    }

    pub fn set_contract_version(&mut self, n: f32) -> () {
        self.contract_version = n.to_string();
    }

    pub fn to_redis_item(&self) -> [(&str, &str); 11] {
        [
            ("txbody", &self.tx_body),
            ("txwitness", &self.tx_witness),
            ("txunsigned", &self.tx_unsigned),
            ("txaux", &self.tx_aux),
            ("txrawdata", &self.tx_raw_data),
            ("txspecific", &self.specific_raw_data),
            ("usedutxos", &self.used_utxos),
            ("stakeaddr", &self.stake_addr),
            ("userid", &self.user_id),
            ("contractid", &self.contract_id),
            ("contractversion", &self.contract_version),
        ]
    }
}

pub fn store_raw_tx(payload: &RawTx) -> Result<String, MurinError> {
    let mut con = redis_txmind_connection()?;

    let key = TxMindId::new(payload);

    let items = payload.to_redis_item();
    let _response: () = redis::cmd("HMSET")
        .arg(&key.id)
        .arg(&items)
        .query(&mut con)?;

    Ok(key.id)
}

pub fn read_raw_tx(key: &String) -> Result<RawTx, MurinError> {
    let mut con = redis_txmind_connection()?;

    let response: HashMap<String, String> = redis::cmd("HGETALL").arg(key).query(&mut con)?;
    let err = MurinError::new(
        &format!("Error in decoding Redis Data for 'RawTx' : {:?}", response).to_string(),
    );

    Ok(RawTx::new(
        response.get("txbody").ok_or(&err)?,
        response.get("txwitness").ok_or(&err)?,
        response.get("txunsigned").ok_or(&err)?,
        response.get("txaux").ok_or(&err)?,
        response.get("txrawdata").ok_or(&err)?,
        response.get("txspecific").ok_or(&err)?,
        response.get("usedutxos").ok_or(&err)?,
        response.get("stakeaddr").ok_or(&err)?,
        &response.get("userid").ok_or(&err)?.parse::<i64>()?,
        &response.get("contractid").ok_or(&err)?.parse::<i64>()?,
        &response
            .get("contractversion")
            .ok_or(&err)?
            .parse::<f32>()?,
    ))
}
