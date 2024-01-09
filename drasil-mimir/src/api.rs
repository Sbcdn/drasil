use super::*;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use drasil_murin::TransactionUnspentOutputs;
use error::MimirError;
use std::ops::{Add, Neg};

/// get all tokens of an utxo
pub fn get_utxo_tokens(
    conn: &mut PgConnection,
    utxo_id: i64,
) -> Result<Vec<UMultiAsset>, MimirError> {
    let multi_assets = multi_asset::table
        .inner_join(ma_tx_out::table.on(multi_asset::id.eq(ma_tx_out::ident)))
        .left_join(unspent_utxos::table.on(ma_tx_out::tx_out_id.eq(unspent_utxos::id)))
        .filter(unspent_utxos::id.eq(utxo_id))
        //.select((multi_asset::id,multi_asset::policy,multi_asset::name,multi_asset::fingerprint))
        .select((
            multi_asset::id,
            multi_asset::policy,
            multi_asset::name,
            multi_asset::fingerprint,
            ma_tx_out::quantity,
        ))
        .load::<UMultiAsset>(conn)?;
    Ok(multi_assets)
}

/// Select the first seen Base Address of a stake address
pub fn select_addr_of_first_transaction(stake_address_in: &str) -> Result<String, MimirError> {
    log::debug!(
        "Try to find first address used by this stake address: {}",
        stake_address_in
    );
    let mut conn = establish_connection()?;
    let resp = tx_out::table
        .left_join(tx::table.on(tx_out::tx_id.eq(tx::id)))
        .left_join(block::table.on(tx::block_id.eq(block::id)))
        .left_join(
            stake_address::table.on(tx_out::stake_address_id.eq(stake_address::id.nullable())),
        )
        .filter(stake_address::view.eq(stake_address_in))
        .select(tx_out::address)
        .order(block::slot_no.asc())
        .first::<String>(&mut conn);
    log::debug!("Found address: {:?}", resp);
    let resp = resp?;
    Ok(resp)
}

/// get all utxos of an address
pub fn get_address_utxos(addr: &String) -> Result<TransactionUnspentOutputs, MimirError> {
    let unspent = unspent_utxos::table
        .filter(unspent_utxos::address.eq(addr))
        .load::<UnspentUtxo>(&mut establish_connection()?)?;
    let mut utxos = TransactionUnspentOutputs::new();
    for u in unspent {
        utxos.add(&u.to_txuo(&mut establish_connection()?)?);
    }
    Ok(utxos)
}

/// Get all utxos of a stake address
pub fn get_stake_address_utxos(
    conn: &mut PgConnection,
    stake_addr: &String,
) -> Result<TransactionUnspentOutputs, MimirError> {
    let unspent = unspent_utxos::table
        .filter(unspent_utxos::stake_address.eq(stake_addr))
        .filter(unspent_utxos::address_has_script.eq(false))
        .load::<UnspentUtxo>(conn)?;
    let mut utxos = TransactionUnspentOutputs::new();
    for u in unspent {
        utxos.add(&u.to_txuo(conn)?);
    }

    Ok(utxos)
}

/// Get all utxos of a stake address
pub fn get_asset_utxos_on_addr(
    conn: &mut PgConnection,
    addr: &String,
) -> Result<TransactionUnspentOutputs, MimirError> {
    let unspent_assets: Vec<UnspentUtxo> = unspent_utxos::table
        .inner_join(ma_tx_out::table.on(ma_tx_out::tx_out_id.eq(unspent_utxos::id)))
        .inner_join(multi_asset::table.on(multi_asset::id.eq(ma_tx_out::ident)))
        .select((
            unspent_utxos::id,
            unspent_utxos::tx_id,
            unspent_utxos::hash,
            unspent_utxos::index,
            unspent_utxos::address,
            unspent_utxos::value,
            unspent_utxos::data_hash,
            unspent_utxos::address_has_script,
            unspent_utxos::stake_address,
        ))
        .filter(unspent_utxos::address.eq(addr))
        .load::<UnspentUtxo>(conn)?;
    let con = &mut establish_connection()?;
    let mut utxos = TransactionUnspentOutputs::new();
    unspent_assets.iter().for_each(|n| {
        utxos.add(
            &n.to_txuo(con)
                .expect("Could not convert into TransactionUnspentOutput"),
        )
    });

    Ok(utxos)
}

/// Get the current slot number
pub fn get_slot(conn: &mut PgConnection) -> Result<i64, MimirError> {
    let slot = block::table
        .filter(block::block_no.is_not_null())
        .select(block::slot_no)
        .order(block::slot_no.desc())
        .limit(1)
        .load::<Option<i64>>(conn)?;
    match slot[0] {
        Some(s) => Ok(s),
        None => Err(MimirError::Custom(
            "ERROR: Could not find slot number in DBsync".to_string(),
        )),
    }
}

/// get total ada staked in an epoch for a stakepool
pub fn get_tot_stake_per_pool(
    conn: &mut PgConnection,
    pool: &String,
    epoch: i32,
) -> Result<Vec<EpochStakeView>, MimirError> {
    let pool_stake = epoch_stake::table
        .inner_join(pool_hash::table.on(pool_hash::id.eq(epoch_stake::pool_id)))
        .inner_join(stake_address::table.on(epoch_stake::addr_id.eq(stake_address::id)))
        .filter(pool_hash::view.eq(pool))
        .filter(epoch_stake::epoch_no.eq(epoch))
        .select((stake_address::view, epoch_stake::amount))
        .load::<EpochStakeView>(conn)?;
    Ok(pool_stake)
}

/// Retrieve delegations for a stakepool
pub fn get_delegations_per_pool_for_epochs(
    conn: &mut PgConnection,
    pool: &String,
    start_epoch: i64,
    end_epoch: i64,
) -> Result<Vec<DelegationView>, MimirError> {
    let deleg = delegation::table
        .inner_join(pool_hash::table.on(pool_hash::id.eq(delegation::pool_hash_id)))
        .inner_join(stake_address::table.on(delegation::addr_id.eq(stake_address::id)))
        .inner_join(tx::table.on(delegation::tx_id.eq(tx::id)))
        .filter(pool_hash::view.eq(pool))
        .filter(delegation::active_epoch_no.ge(start_epoch))
        .filter(delegation::active_epoch_no.le(end_epoch))
        .select((
            stake_address::view,
            tx::deposit,
            delegation::cert_index,
            delegation::active_epoch_no,
        ))
        .load::<DelegationView>(conn)?;
    Ok(deleg)
}

/// get total ada staked for a given epoch
pub fn get_pool_total_stake(
    conn: &mut PgConnection,
    pool: &String,
    epoch: i32,
) -> Result<u64, MimirError> {
    let pool_stake = epoch_stake::table
        .inner_join(pool_hash::table.on(pool_hash::id.eq(epoch_stake::pool_id)))
        .filter(pool_hash::view.eq(pool))
        .filter(epoch_stake::epoch_no.eq(epoch))
        .select(epoch_stake::amount)
        .load::<BigDecimal>(conn)?;

    let tot_stake: u64 = pool_stake.iter().map(|x| x.to_u64().unwrap()).sum();

    Ok(tot_stake)
}

/// get current epoch number
pub fn get_epoch(conn: &mut PgConnection) -> Result<i32, MimirError> {
    let epoch = epoch_stake::table
        .filter(epoch_stake::epoch_no.is_not_null())
        .select(epoch_stake::epoch_no)
        .order(epoch_stake::epoch_no.desc())
        .first::<i32>(conn)?;

    Ok(epoch)
}

/// Get fingerprint from dbsync for a given token
pub fn get_fingerprint(
    conn: &mut PgConnection,
    policy: &String,
    tokenname: &String,
) -> Result<String, MimirError> {
    let fingerprint = multi_asset::table
        .filter(multi_asset::policy.eq(hex::decode(policy)?))
        .filter(multi_asset::name.eq(tokenname.as_bytes()))
        .select(multi_asset::fingerprint)
        .first::<String>(conn)?;

    Ok(fingerprint)
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub policy: String,
    pub tokenname: String,
    pub fingerprint: String,
}

pub fn get_token_info(
    conn: &mut PgConnection,
    fingerprint_in: &String,
) -> Result<TokenInfo, MimirError> {
    let fingerprint = multi_asset::table
        .filter(multi_asset::fingerprint.eq(fingerprint_in))
        .select((multi_asset::policy, multi_asset::name))
        .first::<(Vec<u8>, Vec<u8>)>(conn)?;

    let policy = hex::encode(fingerprint.0);
    let tokenname = hex::encode(fingerprint.1);

    let ti = TokenInfo {
        policy,
        tokenname,
        fingerprint: fingerprint_in.clone(),
    };

    Ok(ti)
}

#[allow(clippy::type_complexity)]
pub fn stake_registration(
    conn: &mut PgConnection,
    stake_addr_in: &str,
) -> Result<Vec<(String, Vec<u8>, i32, i32)>, MimirError> {
    let registration = stake_registration::table
        .inner_join(stake_address::table.on(stake_registration::addr_id.eq(stake_address::id)))
        .inner_join(tx::table.on(stake_registration::tx_id.eq(tx::id)))
        .filter(stake_address::view.eq(stake_addr_in.to_string()))
        .select((
            stake_address::view,
            tx::hash,
            stake_registration::cert_index,
            stake_registration::epoch_no,
        ))
        .order(stake_registration::epoch_no.desc())
        .load::<(String, Vec<u8>, i32, i32)>(conn)?;

    Ok(registration)
}

#[allow(clippy::type_complexity)]
pub fn stake_deregistration(
    conn: &mut PgConnection,
    stake_addr_in: &str,
) -> Result<Vec<(String, Vec<u8>, i32, i32, Option<i64>)>, MimirError> {
    let deregistration = stake_deregistration::table
        .inner_join(stake_address::table.on(stake_deregistration::addr_id.eq(stake_address::id)))
        .inner_join(tx::table.on(stake_deregistration::tx_id.eq(tx::id)))
        .filter(stake_address::view.eq(stake_addr_in.to_string()))
        .select((
            stake_address::view,
            tx::hash,
            stake_deregistration::cert_index,
            stake_deregistration::epoch_no,
            stake_deregistration::redeemer_id,
        ))
        .order(stake_deregistration::epoch_no.desc())
        .load::<(String, Vec<u8>, i32, i32, Option<i64>)>(conn)?;

    Ok(deregistration)
}

pub fn check_stakeaddr_registered(stake_addr_in: &str) -> Result<bool, MimirError> {
    let mut conn = crate::establish_connection()?;

    let registration = stake_registration::table
        .inner_join(stake_address::table.on(stake_registration::addr_id.eq(stake_address::id)))
        .inner_join(tx::table.on(stake_registration::tx_id.eq(tx::id)))
        .filter(stake_address::view.eq(stake_addr_in.to_string()))
        .select((
            stake_address::view,
            tx::hash,
            stake_registration::cert_index,
            stake_registration::epoch_no,
        ))
        .order(stake_registration::epoch_no.desc())
        .load::<(String, Vec<u8>, i32, i32)>(&mut conn)?;

    let deregistration = stake_deregistration::table
        .inner_join(stake_address::table.on(stake_deregistration::addr_id.eq(stake_address::id)))
        .inner_join(tx::table.on(stake_deregistration::tx_id.eq(tx::id)))
        .filter(stake_address::view.eq(stake_addr_in.to_string()))
        .select((
            stake_address::view,
            tx::hash,
            stake_deregistration::cert_index,
            stake_deregistration::epoch_no,
        ))
        .order(stake_deregistration::epoch_no.desc())
        .load::<(String, Vec<u8>, i32, i32)>(&mut conn)?;

    match registration.len() {
        0 => Ok(false),
        _ => match deregistration.len() {
            0 => Ok(true),
            _ => {
                if registration[0].3 > deregistration[0].3 {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        },
    }
}

#[derive(Debug, Clone)]
pub struct EligableWallet {
    pub stake_address: String,
    pub hodl_amount: u64,
    pub assetname: Option<String>,
}

pub fn lookup_token_holders(
    fingerprint_in: &String,
    min_amount: Option<&i64>,
) -> Result<Vec<EligableWallet>, MimirError> {
    let mut conn = crate::establish_connection()?;

    let mut holders = unspent_utxos::table
        .inner_join(ma_tx_out::table.on(unspent_utxos::id.eq(ma_tx_out::tx_out_id)))
        .left_join(multi_asset::table.on(multi_asset::id.eq(ma_tx_out::ident)))
        .filter(multi_asset::fingerprint.eq(fingerprint_in))
        .filter(unspent_utxos::stake_address.is_not_null())
        .select((unspent_utxos::stake_address.nullable(), ma_tx_out::quantity))
        .load::<(Option<String>, BigDecimal)>(&mut conn)?;

    if let Some(amt) = min_amount {
        let a = BigDecimal::from_i64(*amt).unwrap();
        holders.retain(|p| p.1 >= a && p.0.is_some())
    } else {
        holders.retain(|p| p.0.is_some())
    }

    let mut ret = Vec::<EligableWallet>::new();
    ret.extend(holders.iter().map(|p| EligableWallet {
        stake_address: p.0.as_ref().unwrap().to_string(),
        hodl_amount: BigDecimal::to_u64(&p.1).unwrap(),
        assetname: None,
    }));

    Ok(ret)
}

pub fn lookup_nft_token_holders(policy: &String) -> Result<Vec<EligableWallet>, MimirError> {
    let mut conn = crate::establish_connection()?;

    let pbyte = hex::decode(policy)?;

    let mut holders = unspent_utxos::table
        .inner_join(ma_tx_out::table.on(unspent_utxos::id.eq(ma_tx_out::tx_out_id)))
        .left_join(multi_asset::table.on(multi_asset::id.eq(ma_tx_out::ident)))
        .filter(multi_asset::policy.eq(pbyte))
        .filter(unspent_utxos::stake_address.is_not_null())
        .select((unspent_utxos::stake_address.nullable(), ma_tx_out::quantity))
        .load::<(Option<String>, BigDecimal)>(&mut conn)?;

    holders.retain(|p| p.0.is_some());

    let mut ret = Vec::<EligableWallet>::new();
    ret.extend(holders.iter().map(|p| EligableWallet {
        stake_address: p.0.as_ref().unwrap().to_string(),
        hodl_amount: BigDecimal::to_u64(&p.1).unwrap(),
        assetname: None,
    }));

    Ok(ret)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TokenInfoMint {
    pub fingerprint: String,
    pub policy: String,
    pub tokenname: String,
    pub meta_key: i64,
    pub json: Option<serde_json::Value>,
    pub txhash: String,
}

pub fn get_mint_metadata(fingerprint_in: &str) -> Result<TokenInfoMint, MimirError> {
    let metadata = ma_tx_mint::table
        .inner_join(multi_asset::table.on(multi_asset::id.eq(ma_tx_mint::ident)))
        .inner_join(tx_metadata::table.on(tx_metadata::tx_id.eq(ma_tx_mint::tx_id)))
        .inner_join(tx::table.on(ma_tx_mint::tx_id.eq(tx::id)))
        .inner_join(block::table.on(tx::block_id.eq(block::id)))
        .filter(multi_asset::fingerprint.eq(fingerprint_in.to_string()))
        .order_by(block::slot_no.desc())
        .select((
            multi_asset::fingerprint,
            multi_asset::policy,
            multi_asset::name,
            tx_metadata::key,
            tx_metadata::json.nullable(),
            tx::hash,
        ))
        .first::<(
            String,
            Vec<u8>,
            Vec<u8>,
            BigDecimal,
            Option<serde_json::Value>,
            Vec<u8>,
        )>(&mut crate::establish_connection()?)
        .map_err(|_| MimirError::NotOnChainMetadataFound)?;

    let tokenname =
        String::from_utf8(metadata.2.clone()).unwrap_or(hex::encode(metadata.2.clone()));
    Ok(TokenInfoMint {
        fingerprint: metadata.0,
        policy: hex::encode(metadata.1),
        tokenname,
        meta_key: metadata.3.to_i64().unwrap(),
        json: metadata.4,
        txhash: hex::encode(metadata.5),
    })
}

/*
pub fn lookup_mint_metadata_condition(
    policy  : &String,
) -> Result<Option<Vec::<EligableWallet>>,MimirError> {
    let conn = crate::establish_connection()?;

    let pbyte = hex::decode(&policy)?;

    // Check for all "latests" mint transactions which contain a token with the given policy ID
    // where the metadata of the minting transaction contain the given trait
    // Return all matching NFTs (Tokens)

    // Second Step
    // Lookup current holders of those Tokens and return stake addresses


    let mut holders = unspent_utxos::table
                    .left_join(ma_tx_out::table.on(unspent_utxos::id.eq(ma_tx_out::tx_out_id)))
                    .left_join(multi_asset::table.on(multi_asset::id.eq(ma_tx_out::ident)))
                    .filter(multi_asset::policy.eq(pbyte))
                    .filter(unspent_utxos::stake_address.is_not_null())
                    .filter(unspent_utxos::address_has_script.eq(false))
                    .filter(ma_tx_out::quantity.eq(1))
                    //.select((multi_asset::id,multi_asset::policy,multi_asset::name,multi_asset::fingerprint))
                    .select((unspent_utxos::stake_address.nullable(), multi_asset::name) )
                    .load::<(Option<String>, BigDecimal, Vec::<u8>)>(&conn)?;

}
*/

pub fn find_avail_pool(pool_id: &String) -> Result<bool, MimirError> {
    let mut conn = establish_connection()?;
    let pool_stake = pool_hash::table
        .filter(pool_hash::view.eq(pool_id))
        .first::<PoolHash>(&mut conn)?;

    let pool_retire = pool_retire::table
        .filter(pool_retire::id.eq(&pool_stake.id))
        .load::<PoolRetire>(&mut conn)?;

    if !pool_retire.is_empty() {
        return Ok(false);
    }

    Ok(true)
}

pub async fn txhash_is_spent(txhash: &String) -> Result<bool, MimirError> {
    let mut conn = establish_connection()?;
    let txh_b = hex::decode(txhash)?;
    let tx = tx_out::table
        .inner_join(tx::table.on(tx::id.eq(tx_out::tx_id)))
        .left_join(
            tx_in::table.on(tx_in::tx_out_id
                .eq(tx::id)
                .and(tx_in::tx_out_index.eq(tx_out::index))),
        )
        .select((tx::hash, tx_out::index))
        .filter(tx_in::tx_in_id.is_not_null())
        .filter(tx::hash.eq(txh_b))
        .load::<(Vec<u8>, i16)>(&mut conn)?;
    if !tx.is_empty() {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// The sum of all rewards ever received by the given stake address.
pub async fn total_rewards(stake_addr: &str) -> Result<BigDecimal, MimirError> {
    Ok(reward::table
        .inner_join(stake_address::table.on(stake_address::id.eq(reward::addr_id)))
        .filter(stake_address::view.eq(stake_addr.to_string()))
        .select(reward::amount)
        .load::<BigDecimal>(&mut establish_connection()?)?
        .iter()
        .sum())
}

/// The sum of all reward withdrawals ever made by the given stake address.
pub async fn total_withdrawals(stake_addr: &str) -> Result<BigDecimal, MimirError> {
    let response = withdrawal::table
        .inner_join(stake_address::table.on(stake_address::id.eq(withdrawal::addr_id)))
        .filter(stake_address::view.eq(stake_addr.to_string()))
        .select(withdrawal::amount)
        .load::<BigDecimal>(&mut establish_connection()?)?
        .iter()
        .sum();
    Ok(response)
}

/// The sum of all delegations (deposits) ever made by the given stake address.
pub async fn total_deposits(stake_addr: &str) -> Result<BigDecimal, MimirError> {
    let response = delegation::table
        .inner_join(tx::table.on(tx::id.eq(delegation::tx_id)))
        .inner_join(stake_address::table.on(stake_address::id.eq(delegation::addr_id)))
        .filter(stake_address::view.eq(stake_addr.to_string()))
        .select(tx::out_sum)
        .load::<BigDecimal>(&mut establish_connection()?)?
        .iter()
        .sum();
    Ok(response)
}

/// The total amount of rewards that the given stake address
/// has earned but not withdrawn.
pub async fn withdrawable_rewards(stake_addr: &str) -> Result<BigDecimal, MimirError> {
    let response = reward::table
        .inner_join(stake_address::table.on(stake_address::id.eq(reward::addr_id)))
        .filter(stake_address::view.eq(stake_addr.to_string()))
        .select(reward::amount)
        .load::<BigDecimal>(&mut establish_connection()?)?
        .iter()
        .sum::<BigDecimal>()
        .add(
            withdrawal::table
                .inner_join(stake_address::table.on(stake_address::id.eq(withdrawal::addr_id)))
                .filter(stake_address::view.eq(stake_addr.to_string()))
                .select(withdrawal::amount)
                .load::<BigDecimal>(&mut establish_connection()?)?
                .iter()
                .sum::<BigDecimal>()
                .neg(),
        );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use crate::TokenInfoMint;
    use bigdecimal::BigDecimal;
    use dotenv::dotenv;
    use serde_json::json;
    use std::ops::{Add, Neg};
    use std::str::FromStr;
    use tokio;

    #[test]
    fn stake_registration() {
        dotenv().ok();
        let mut conn = crate::establish_connection().unwrap();
        let stake_addr_in = "stake_test1uqnfwu6xlrp95yhkzq0q5p3ct2adrrt92vx5yqsr4ptqkugn5s708";
        let func_value = super::stake_registration(&mut conn, stake_addr_in).unwrap();

        let (
            stake_address_view,
            tx_hash,
            stake_registration_cert_index,
            stake_registration_epoch_no,
        ) = func_value[0].clone();

        assert_eq!(func_value.len(), 1);
        assert_eq!(stake_address_view, stake_addr_in.to_string());
        assert_eq!(
            hex::encode(tx_hash.clone()),
            "c6a5fb39114863da737a824f355c09f720479a94bff91b3613639e7313128dc7".to_string()
        );
        assert_eq!(stake_registration_cert_index, 0);
        assert_eq!(stake_registration_epoch_no, 38);
    }

    #[test]
    fn stake_deregistration() {
        dotenv().ok();
        let mut conn = crate::establish_connection().unwrap();
        let stake_addr_in = "stake_test1urtyeyl0qz20tsteu5uqzz0tamczyfzegn3ezn6mej360ycky7cg5";
        let func_value = super::stake_deregistration(&mut conn, stake_addr_in).unwrap();

        assert_eq!(func_value.len(), 94);

        let (
            stake_address_view,
            tx_hash,
            stake_deregistration_cert_index,
            stake_deregistration_epoch_no,
            stake_deregistration_redeemer_id,
        ) = func_value.last().unwrap();

        assert_eq!(stake_address_view, stake_addr_in);
        assert_eq!(
            hex::encode(tx_hash),
            "49d55da2db879398172ada91da2f02d902a51daf744b5ff6da8c0c96467c0c2a"
        ); // assumed to be correct
        assert_eq!(stake_deregistration_cert_index, &0);
        assert_eq!(stake_deregistration_epoch_no, &8);
        assert_eq!(stake_deregistration_redeemer_id, &None);
    }

    #[test]
    fn check_stakeaddr_registered() {
        dotenv().ok();
        let stake_addr_in = "stake_test1uqnfwu6xlrp95yhkzq0q5p3ct2adrrt92vx5yqsr4ptqkugn5s708";
        let func_value = super::check_stakeaddr_registered(stake_addr_in).unwrap();
        let real_value = true;

        assert_eq!(func_value, real_value);
    }

    #[test]
    fn get_mint_metadata() {
        let fingerprint_in = "asset1t3m9e0gysc4xy392q25qwgj2hwca5qf3nhgcf4";
        let func_value = super::get_mint_metadata(fingerprint_in).unwrap();
        let real_value = TokenInfoMint {
            fingerprint: "asset1t3m9e0gysc4xy392q25qwgj2hwca5qf3nhgcf4".to_string(),
            policy: "f0ff48bbb7bbe9d59a40f1ce90e9e9d0ff5002ec48f232b49ca0fb9a".to_string(),
            tokenname: "000643b0766963746f722e656c6261".to_string(),
            meta_key: 721,
            json: Some(
                json!({"f0ff48bbb7bbe9d59a40f1ce90e9e9d0ff5002ec48f232b49ca0fb9a": {"000de140766963746f722e656c6261": {"og": 0, "name": "$victor.elba", "image": "ipfs://zb2rhoGr5TMRmSYhupacB5pVSbViLWrt2MyTs1DH8i8ArRQAU", "length": 11, "rarity": "basic", "version": 1, "mediaType": "image/jpeg", "og_number": 0, "characters": "letters,special", "numeric_modifiers": ""}}}),
            ),
            txhash: "fcc86f8adb7349eca89ec75a79be19091fb3040bf94baf95fdd421a625eaf045".to_string(),
        };

        assert_eq!(func_value.fingerprint, real_value.fingerprint);
        assert_eq!(func_value.policy, real_value.policy);
        assert_eq!(func_value.tokenname, real_value.tokenname);
        assert_eq!(func_value.meta_key, real_value.meta_key);
        assert_eq!(func_value.json, real_value.json);
        assert_eq!(func_value.txhash, real_value.txhash);
    }

    #[tokio::test]
    async fn total_rewards() {
        dotenv().ok();
        let stake_addr = "stake_test1urjuh5w4xxlma4pauruygvraqsuv5r8rctsnh9jj7n6ff0gs6zhde";
        let _func_value = super::total_rewards(stake_addr).await.unwrap();
        let _manual_value = BigDecimal::from(19800282);
    }

    #[tokio::test]
    async fn total_withdrawals() {
        dotenv().ok();
        let stake_addr = "stake_test1upvv3c4l2jfhkannqf3lp4htmqvpscdsmhvyhalaecj3jdqtfcgvh";
        let _func_value = super::total_withdrawals(stake_addr).await.unwrap();
        let _manual_value = BigDecimal::from(1974883422);
    }

    #[tokio::test]
    async fn total_deposits() {
        dotenv().ok();
        let stake_addr = "stake_test1upvv3c4l2jfhkannqf3lp4htmqvpscdsmhvyhalaecj3jdqtfcgvh";
        let _func_value = super::total_deposits(stake_addr).await.unwrap();
        let _manual_value = BigDecimal::from_str("15552531005").unwrap();
    }

    #[tokio::test]
    async fn withdrawable_rewards() {
        dotenv().ok();
        let stake_addr = "stake_test1upvv3c4l2jfhkannqf3lp4htmqvpscdsmhvyhalaecj3jdqtfcgvh";
        let func_value = super::withdrawable_rewards(stake_addr).await.unwrap();
        let manual_value = super::total_rewards(stake_addr)
            .await
            .unwrap()
            .add(super::total_withdrawals(stake_addr).await.unwrap().neg());
        assert_eq!(func_value, manual_value);
    }
}
