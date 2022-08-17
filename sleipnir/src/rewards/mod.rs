/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
pub use crate::error::SleipnirError;

use chrono::{NaiveDateTime, Utc};
use hugin::database::*;
use murin::*;
use serde_json::json;
use std::str::FromStr;

pub async fn create_contract(
    network: murin::clib::NetworkIdKind,
    user_id: i64,
    fee: Option<i64>,
) -> Result<i64, SleipnirError> {
    // create rwd wallet  & contract

    let mut net_bytes = 0b0001;
    if network == murin::clib::NetworkIdKind::Testnet {
        net_bytes = 0b0000;
    }

    // create key1
    let root_key1: clib::crypto::Bip32PrivateKey =
        clib::crypto::Bip32PrivateKey::generate_ed25519_bip32()?;
    let pvk1_root_bytes = hex::encode(root_key1.as_bytes());
    let account_key1 = root_key1
        .derive(harden(1852u32))
        .derive(harden(1815u32))
        .derive(harden(0u32));
    let ac1_chaincode = account_key1.chaincode();
    let ac1_private_key = account_key1.to_raw_key(); // for signatures
    let ac1_publick_key = account_key1.to_raw_key().to_public();
    let ac1_public_key_hash = account_key1.to_raw_key().to_public().hash(); // for Native Script Input / Verification
    let _vkey1 = "5840".to_string()
        + &((hex::encode(ac1_publick_key.as_bytes())) + &hex::encode(ac1_chaincode.clone())); // .vkey
    let _skey1 = "5880".to_string()
        + &(hex::encode(ac1_private_key.as_bytes())
            + &hex::encode(ac1_publick_key.as_bytes())
            + &hex::encode(ac1_chaincode)); // .vkey

    // create key2
    let root_key2: clib::crypto::Bip32PrivateKey =
        clib::crypto::Bip32PrivateKey::generate_ed25519_bip32()?;
    let pvk2_root_bytes = hex::encode(root_key2.as_bytes());
    let account_key2 = root_key2
        .derive(harden(1852u32))
        .derive(harden(1815u32))
        .derive(harden(0u32));
    let ac2_chaincode = account_key2.chaincode();
    let ac2_private_key = account_key2.to_raw_key(); // for signatures
    let ac2_publick_key = account_key2.to_raw_key().to_public();
    let ac2_public_key_hash = account_key2.to_raw_key().to_public().hash(); // for Native Script Input / Verification
    let _vkey2 = "5840".to_string()
        + &((hex::encode(ac2_publick_key.as_bytes())) + &hex::encode(ac2_chaincode.clone())); // .vkey
    let _skey2 = "5880".to_string()
        + &(hex::encode(ac2_private_key.as_bytes())
            + &hex::encode(ac2_publick_key.as_bytes())
            + &hex::encode(ac2_chaincode)); // .vkey

    let mut native_scripts = NativeScripts::new();
    native_scripts.add(&NativeScript::new_script_pubkey(&ScriptPubkey::new(
        &ac1_public_key_hash,
    )));
    native_scripts.add(&NativeScript::new_script_pubkey(&ScriptPubkey::new(
        &ac2_public_key_hash,
    )));

    let rwd_script = NativeScript::new_script_all(&ScriptAll::new(&native_scripts));
    let script_hash = rwd_script.hash(); //policyId

    let stake_creds = clib::address::StakeCredential::from_scripthash(&script_hash);
    let script_address_e =
        clib::address::EnterpriseAddress::new(net_bytes, &stake_creds).to_address();
    let sc_address_bech32 = script_address_e.to_bech32(None)?;
    let d = &format!(
        "RWD Multi Signature Native Script user: {:?}",
        user_id.clone()
    )[..];
    let description = Some(d);
    let contract_id = TBContracts::get_next_contract_id(&user_id)?;

    let contract_type = "sporwc";

    let _ = TBContracts::create_contract(
        &user_id,
        &contract_id,
        contract_type,
        description,
        &0.1,
        &hex::encode(rwd_script.to_bytes()),
        &sc_address_bech32,
        Some(&hex::encode(script_hash.to_bytes())),
        &false,
    )?;

    let pvks = vec![pvk1_root_bytes, pvk2_root_bytes];

    let _kl = TBMultiSigLoc::create_multisig_keyloc(
        &user_id,
        &contract_id,
        &0.1,
        &sc_address_bech32,
        Some(&sc_address_bech32),
        fee.as_ref(),
        &pvks,
        &false,
    )
    .await?;

    Ok(contract_id)
}

pub async fn depricate_contract(
    user_id: i64,
    contract_id: i64,
) -> Result<serde_json::Value, SleipnirError> {
    let dconn = hugin::database::establish_connection()?;
    let resp = hugin::TBContracts::depricate_contract(&dconn, &(user_id), &(contract_id), &true)?;

    Ok(json!(resp))
}

#[derive(Debug, Clone, serde::Serialize)]
struct ViewAdmContracts {
    pub user_id: i64,
    pub contract_id: i64,
    pub description: Option<String>,
    pub address: String,
    pub depricated: bool,
    pub drasil_lqdty: Option<i64>,
    pub customer_lqdty: Option<i64>,
    pub external_lqdty: Option<i64>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub tx_month: i64,
    pub tx_total: i64,
}

pub async fn get_rwd_contracts_for_user(user_id: i64) -> Result<serde_json::Value, SleipnirError> {
    use chrono::Datelike;
    use chrono::TimeZone;
    let contracts =
        hugin::TBContracts::get_all_contracts_for_user_typed(user_id, "sporwc".to_string())?;

    let current_date = chrono::Utc::now();
    let first: chrono::NaiveDateTime =
        chrono::NaiveDate::from_ymd(current_date.year(), current_date.month(), 1)
            .and_hms(00, 00, 00);
    let date_time: chrono::DateTime<Utc> = chrono::Utc.from_local_datetime(&first).unwrap();

    let mut resp = Vec::<ViewAdmContracts>::new();

    resp.extend(contracts.iter().map(|c| {
        let all_tx =
            gungnir::Claimed::get_stat_count_all_tx_on_contr(c.contract_id, user_id, None).unwrap();
        let month_tx = gungnir::Claimed::get_stat_count_period_tx_contr_token(
            None,
            c.contract_id,
            user_id,
            date_time,
            current_date,
        )
        .unwrap();

        ViewAdmContracts {
            user_id: c.user_id,
            contract_id: c.contract_id,
            description: c.description.clone(),
            address: c.address.clone(),
            depricated: c.depricated,
            drasil_lqdty: c.drasil_lqdty,
            customer_lqdty: c.customer_lqdty,
            external_lqdty: c.external_lqdty,
            created_at: c.created_at,
            updated_at: c.updated_at,
            tx_month: month_tx,
            tx_total: all_tx,
        }
    }));

    Ok(json!(resp))
}

pub async fn reactivate_contract(
    user_id: i64,
    contract_id: i64,
) -> Result<serde_json::Value, SleipnirError> {
    let dconn = hugin::database::establish_connection()?;
    let resp = hugin::TBContracts::depricate_contract(&dconn, &(user_id), &(contract_id), &false)?;

    Ok(json!(resp))
}

pub fn create_token_whitelisting(
    user_id: i64,
    contract_id: i64,
    fingerprint: String,
    vesting_period: Option<String>,
    pools: Option<Vec<String>>,
    mode: String,
    equation: String,
    start_epoch_in: i64,
    end_epoch: Option<i64>,
    modificator_equ: Option<String>,
) -> Result<serde_json::Value, SleipnirError> {
    log::debug!("Process vesting period...");
    let mut vd = chrono::Utc::now();
    if let Some(date) = vesting_period {
        vd = chrono::DateTime::<Utc>::from_utc(
            NaiveDateTime::parse_from_str(&date, "%Y-%m-%d %H:%M:%S")?,
            Utc,
        );
    }
    log::debug!("Retrieve token information...");
    let mconn = mimir::establish_connection()?;
    let ti = mimir::get_token_info(&mconn, &fingerprint)?;

    //Check Data is valid
    // Epochs
    log::debug!("Process epochs...");
    let mconn = mimir::establish_connection()?;
    let current_epoch = mimir::get_epoch(&mconn)? as i64;
    let mut start_epoch = start_epoch_in;
    if start_epoch < current_epoch {
        log::error!(
            "Start epoch cannot be smaller as current epoch, current epoch set as start epoch..."
        );
        start_epoch = current_epoch;
        //return Err(SleipnirError::new(&format!("Start epoch: {} cannot be smaller than the current epoch: : {:?}",start_epoch, current_epoch)))
    }
    if let Some(endepoch) = end_epoch {
        if endepoch <= current_epoch || endepoch <= start_epoch {
            return Err(SleipnirError::new(&format!(
                "End epoch: {}, needs to be in future and after start epoch: {:?}",
                endepoch, start_epoch
            )));
        }
    }
    log::debug!("Process pools...");
    // Pools
    let mut spools = Vec::<gungnir::GPools>::new();
    if let Some(ps) = pools {
        for pool in &ps {
            if !mimir::find_avail_pool(pool)? {
                return Err(SleipnirError::new(&format!(
                    "One of the pools is not existing or retired: {}",
                    pool
                )));
            }
        }
        spools.extend(ps.iter().map(|p| gungnir::GPools {
            pool_id: p.clone(),
            first_valid_epoch: current_epoch,
        }));
    }

    //Mode
    log::debug!("Process calculation mode...");
    match gungnir::Calculationmode::from_str(&mode)? {
        gungnir::Calculationmode::FixedEndEpoch => {
            equation.parse::<u64>()?;
        }
        gungnir::Calculationmode::RelationalToADAStake => {
            equation.parse::<f32>()?;
        }
        gungnir::Calculationmode::Custom => {}
        gungnir::Calculationmode::AirDrop => {}
        _ => {
            return Err(SleipnirError::new(&format!(
                "Calculation Mode is invalid: {:?}",
                mode
            )))
        }
    }

    log::debug!("Process modificator equiation...");
    if modificator_equ.is_some() {
        return Err(SleipnirError::new("Modificator equation not supported yet"));
    }

    // Check token does not already exists -> database constraint ensures this already
    log::debug!("Establish connection to rwd database...");
    let gconn = gungnir::establish_connection()?;
    log::debug!("Try to create twl...");
    let resp = match gungnir::TokenWhitelist::create_twl_entry(
        &gconn,
        &fingerprint,
        &ti.policy,
        &ti.tokenname,
        &(contract_id),
        &(user_id),
        &vd,
        &spools,
        &gungnir::Calculationmode::from_str(&mode)?,
        &equation,
        &start_epoch,
        end_epoch.as_ref(),
        modificator_equ.as_ref(),
    ) {
        Ok(o) => {
            json!(o)
        }
        Err(e) => {
            json!(e.to_string())
        }
    };

    Ok(json!(resp))
}

pub async fn remove_token_whitelisting(
    user_id: i64,
    contract_id: i64,
    fingerprint: String,
) -> Result<serde_json::Value, SleipnirError> {
    let resp = gungnir::TokenWhitelist::remove_twl(&fingerprint, &(contract_id), &(user_id))?;

    Ok(json!(resp))
}

pub async fn get_pools(
    user_id: i64,
    contract_id: i64,
    fingerprint: String,
) -> Result<serde_json::Value, SleipnirError> {
    let resp = gungnir::TokenWhitelist::get_pools(&fingerprint, &(contract_id), &(user_id))?;

    Ok(json!(resp))
}

pub async fn add_pools(
    user_id: i64,
    contract_id: i64,
    fingerprint: String,
    pools: Vec<String>,
) -> Result<serde_json::Value, SleipnirError> {
    // Pools
    let mconn = mimir::establish_connection()?;
    let current_epoch = mimir::get_epoch(&mconn)? as i64;
    let mut spools = Vec::<gungnir::GPools>::new();
    spools.extend(pools.iter().map(|p| gungnir::GPools {
        pool_id: p.clone(),
        first_valid_epoch: current_epoch,
    }));

    let resp =
        gungnir::TokenWhitelist::add_pools(&fingerprint, &(contract_id), &(user_id), &spools)?;

    Ok(json!(resp))
}

pub async fn rm_pools(
    user_id: i64,
    contract_id: i64,
    fingerprint: String,
    pools: Vec<String>,
) -> Result<serde_json::Value, SleipnirError> {
    let mut spools = Vec::<gungnir::GPools>::new();
    // We can set first valid epoch to zero here as for remove we just compare the pool_ids
    spools.extend(pools.iter().map(|p| gungnir::GPools {
        pool_id: p.clone(),
        first_valid_epoch: 0,
    }));

    let resp =
        gungnir::TokenWhitelist::remove_pools(&fingerprint, &(contract_id), &(user_id), &spools)?;

    Ok(json!(resp))
}

pub async fn get_user_txs(
    user: i64,
    from: Option<String>,
    to: Option<String>,
) -> Result<i64, SleipnirError> {
    let mut from_t = chrono::Utc::now();
    if let Some(date) = from {
        from_t = chrono::DateTime::<Utc>::from_utc(
            NaiveDateTime::parse_from_str(&date, "%Y-%m-%d %H:%M:%S")?,
            Utc,
        );
    }

    let mut to_t = chrono::Utc::now();
    if let Some(date) = to.clone() {
        to_t = chrono::DateTime::<Utc>::from_utc(
            NaiveDateTime::parse_from_str(&date, "%Y-%m-%d %H:%M:%S")?,
            Utc,
        );
    }

    let mut resp = 0;
    match to {
        Some(_) => {
            let resp1 = gungnir::Claimed::get_stat_count_period_tx_user(&user, &from_t, &to_t);
            println!("{:?}", resp1);
        }
        None => {
            resp = gungnir::Claimed::get_stat_count_all_tx_user(&user)?;
        }
    }

    Ok(resp)
}

pub async fn get_tokens_from_contract(
    user_id: i64,
    contract_id: i64,
) -> Result<serde_json::Value, SleipnirError> {
    let resp = gungnir::TokenWhitelist::get_rwd_contract_tokens(contract_id, user_id)?;

    Ok(json!(resp))
}
