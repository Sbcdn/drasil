/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use super::*;
use crate::error::RWDError;
use crate::schema::*;
use bigdecimal::{FromPrimitive, ToPrimitive};
use std::ops::Div;

impl Rewards {
    pub fn get_rewards_stake_addr(
        conn: &mut PgConnection,
        stake_addr_in: String,
    ) -> Result<Vec<Rewards>, RWDError> {
        use crate::schema::rewards::dsl::*;
        let result = rewards
            .filter(stake_addr.eq(&stake_addr_in))
            .load::<Rewards>(conn)?;
        Ok(result)
    }

    pub fn get_rewards(
        conn: &mut PgConnection,
        stake_addr_in: String,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<Vec<Rewards>, RWDError> {
        use crate::schema::rewards::dsl::*;
        let result = rewards
            .filter(stake_addr.eq(&stake_addr_in))
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .load::<Rewards>(conn)?;
        Ok(result)
    }

    pub fn get_specific_asset_reward(
        conn: &mut PgConnection,
        payment_addr_in: &String,
        fingerprint_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<Vec<Rewards>, RWDError> {
        use crate::schema::rewards::dsl::*;
        let result = rewards
            .filter(payment_addr.eq(&payment_addr_in))
            .filter(fingerprint.eq(&fingerprint_in))
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .load::<Rewards>(conn)?;
        Ok(result)
    }

    pub fn get_avail_specific_asset_reward(
        conn: &mut PgConnection,
        payment_addr_in: &String,
        stake_addr_in: &String,
        fingerprint_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<Vec<Rewards>, RWDError> {
        use crate::schema::rewards::dsl::*;
        let result = rewards
            .filter(payment_addr.eq(&payment_addr_in))
            .filter(fingerprint.eq(&fingerprint_in))
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .filter(tot_earned.gt(tot_claimed))
            .load::<Rewards>(conn)?;

        let mut res = Vec::<Rewards>::new();
        for r in result {
            let claim_sum = Claimed::get_token_claims_tot_amt(
                conn,
                stake_addr_in,
                fingerprint_in,
                contract_id_in,
                user_id_in,
            );
            match claim_sum {
                Ok(cs) => {
                    if cs < r.tot_earned.to_i128().unwrap() {
                        res.push(r);
                    }
                }
                Err(e) => {
                    log::error!(
                        "FOR OBSERVATION: ERROR in claim search for NFT: {:?}",
                        e.to_string()
                    );
                    res.push(r);
                }
            }
        }

        Ok(res)
    }

    pub fn get_rewards_per_token(
        conn: &mut PgConnection,
        stake_addr_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
        fingerprint_in: &String,
    ) -> Result<Vec<Rewards>, RWDError> {
        use crate::schema::rewards::dsl::*;
        let result = rewards
            .filter(stake_addr.eq(&stake_addr_in))
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .filter(fingerprint.eq(&fingerprint_in))
            .load::<Rewards>(conn);
        match result {
            Ok(o) => Ok(o),
            Err(e) => {
                log::error!("Error: {:?}", e.to_string());
                Ok(Vec::<Rewards>::new())
            }
        }
    }

    pub fn get_total_rewards_token(
        user_id_in: i64,
    ) -> Result<Vec<(i64, String, BigDecimal)>, RWDError> {
        use crate::schema::rewards::dsl::*;
        let conn = &mut establish_connection()?;
        let twl = TokenWhitelist::get_user_tokens(&(user_id_in as u64))?;

        let mut out = Vec::<(i64, String, BigDecimal)>::new();

        for i in twl {
            let twl_rewards: Vec<Rewards> = rewards
                .filter(contract_id.eq(&i.contract_id))
                .filter(user_id.eq(&user_id_in))
                .filter(fingerprint.eq(&i.fingerprint.clone().unwrap_or_default()))
                .load::<Rewards>(conn)?;

            let sum: BigDecimal = twl_rewards.iter().fold(
                bigdecimal::FromPrimitive::from_u64(0).unwrap(),
                |acc, n| {
                    acc + (n
                        .tot_earned
                        .clone()
                        .div(&bigdecimal::FromPrimitive::from_u64(1000000).unwrap()))
                        - n.tot_claimed.clone()
                },
            );
            out.push((i.contract_id, i.fingerprint.clone().unwrap(), sum))
        }

        Ok(out)
    }

    pub fn get_available_rewards(
        conn: &mut PgConnection,
        stake_addr_in: &String,
        payment_addr_in: &String,
        fingerprint_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
        claim_request: i128,
    ) -> Result<i128, RWDError> {
        use crate::schema::rewards::dsl::*;
        log::info!("Try to find existing rewards...");
        let result = rewards
            .filter(stake_addr.eq(&stake_addr_in))
            .filter(fingerprint.eq(&fingerprint_in))
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .select((tot_earned, tot_claimed, payment_addr))
            .first::<(BigDecimal, BigDecimal, String)>(conn)?;
        log::info!("found rewards");
        if *payment_addr_in != result.2 {
            return Err(RWDError::new(
                "Reward Error: Missmatching Payment Addresses!",
            ));
        }
        let claim_sum = match Claimed::get_token_claims_tot_amt(
            conn,
            stake_addr_in,
            fingerprint_in,
            contract_id_in,
            user_id_in,
        ) {
            Ok(i) => i,
            Err(e) => {
                if e.to_string() == *"Record not found" {
                    0
                } else {
                    println!("Other error..");
                    return Err(e);
                }
            }
        };
        log::info!("found claims");
        let lovelace = BigDecimal::from_i32(1000000).unwrap();
        match ((result.0 / lovelace) - result.1.clone()).to_i128() {
            Some(dif) => {
                if claim_sum != result.1.to_i128().unwrap() {
                    return Err(RWDError::new(
                        "Error: Missmatch on claimed amount, please contact support!",
                    ));
                }
                Ok(dif - claim_request)
            }
            None => Err(RWDError::new(
                "Could not calculate available tokens for claim",
            )),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_rewards<'a>(
        conn: &mut PgConnection,
        stake_addr: &'a String,
        payment_addr: &'a String,
        fingerprint: &'a String,
        contract_id: &'a i64,
        user_id: &'a i64,
        tot_earned: &'a BigDecimal,
        tot_claimed: &'a BigDecimal,
        oneshot: &'a bool,
        last_calc_epoch: &'a i64,
    ) -> Result<Rewards, RWDError> {
        let new_rewards = RewardsNew {
            stake_addr,
            payment_addr,
            fingerprint,
            contract_id,
            user_id,
            tot_earned,
            tot_claimed,
            oneshot,
            last_calc_epoch,
        };

        Ok(diesel::insert_into(rewards::table)
            .values(&new_rewards)
            .get_result::<Rewards>(conn)?)
    }

    pub fn update_rewards<'a>(
        conn: &mut PgConnection,
        stake_addr_in: &'a String,
        fingerprint_in: &'a String,
        contract_id_in: &'a i64,
        user_id_in: &'a i64,
        tot_earned_in: &'a BigDecimal,
        last_calc_epoch_in: &'a i64,
    ) -> Result<Rewards, RWDError> {
        use crate::schema::rewards::dsl::*;
        let contract = diesel::update(
            rewards
                .filter(stake_addr.eq(stake_addr_in))
                .filter(fingerprint.eq(fingerprint_in))
                .filter(contract_id.eq(contract_id_in))
                .filter(user_id.eq(user_id_in)),
        )
        .set((
            tot_earned.eq(tot_earned_in),
            last_calc_epoch.eq(last_calc_epoch_in),
        ))
        .get_result::<Rewards>(conn)?;

        Ok(contract)
    }

    pub fn update_claimed<'a>(
        conn: &mut PgConnection,
        stake_addr_in: &'a String,
        fingerprint_in: &'a String,
        contract_id_in: &'a i64,
        user_id_in: &'a i64,
        claimed: &'a u64,
    ) -> Result<Rewards, RWDError> {
        use crate::schema::rewards::dsl::*;
        let rwds = Self::get_rewards_per_token(
            conn,
            stake_addr_in,
            *contract_id_in,
            *user_id_in,
            fingerprint_in,
        )?;
        if rwds.len() == 1 {
            let total_claimed =
                rwds[0].tot_claimed.clone() + BigDecimal::from_u64(*claimed).unwrap();
            let contract = diesel::update(
                rewards
                    .filter(stake_addr.eq(stake_addr_in))
                    .filter(fingerprint.eq(fingerprint_in))
                    .filter(contract_id.eq(contract_id_in))
                    .filter(user_id.eq(user_id_in)),
            )
            .set(tot_claimed.eq(total_claimed))
            .get_result::<Rewards>(conn)?;

            Ok(contract)
        } else {
            Err(RWDError::new(&format!(
                "Could not find rewards for Stake Addr: {}, Contract-ID: {}, User-Id: {}",
                stake_addr_in, contract_id_in, user_id_in
            )))
        }
    }

    pub fn get_tot_rewards_to_deliver(
        conn: &mut PgConnection,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<BigDecimal, RWDError> {
        use crate::schema::rewards::dsl::*;
        let ret = rewards
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .select((tot_earned, tot_claimed))
            .load::<(BigDecimal, BigDecimal)>(conn)?;
        let lovelace = BigDecimal::from_i32(1000000).unwrap();
        let open_amt = ret.iter().map(|(x, y)| (x / &lovelace) - y).sum();
        Ok(open_amt)
    }
}

impl Claimed {
    pub fn get_claims(
        conn: &mut PgConnection,
        stake_addr_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<Vec<Claimed>, RWDError> {
        use crate::schema::claimed::dsl::*;
        let result = claimed
            .filter(stake_addr.eq(&stake_addr_in))
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .order(timestamp.asc())
            .load::<Claimed>(conn)?;
        Ok(result)
    }

    pub fn get_all_claims(
        conn: &mut PgConnection,
        stake_addr_in: &String,
    ) -> Result<Vec<Claimed>, RWDError> {
        use crate::schema::claimed::dsl::*;
        let result = claimed
            .filter(stake_addr.eq(&stake_addr_in))
            .order(timestamp.asc())
            .load::<Claimed>(conn)?;
        Ok(result)
    }

    pub fn get_token_claims(
        conn: &mut PgConnection,
        stake_addr_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
        fingerprint_in: &String,
    ) -> Result<Vec<Claimed>, RWDError> {
        use crate::schema::claimed::dsl::*;
        let result = claimed
            .filter(stake_addr.eq(stake_addr_in))
            .filter(contract_id.eq(contract_id_in))
            .filter(user_id.eq(user_id_in))
            .filter(fingerprint.eq(fingerprint_in))
            .load::<Claimed>(conn)?;
        Ok(result)
    }

    pub fn get_token_claims_tot_amt(
        conn: &mut PgConnection,
        stake_addr_in: &String,
        fingerprint_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<i128, RWDError> {
        use crate::schema::claimed::dsl::*;
        let result = claimed
            .filter(stake_addr.eq(stake_addr_in))
            .filter(contract_id.eq(contract_id_in))
            .filter(user_id.eq(user_id_in))
            .filter(fingerprint.eq(fingerprint_in))
            .filter(invalid.is_null())
            .select(amount)
            .load::<BigDecimal>(conn)?;

        let sum = result.iter().map(|x| x.to_i128().unwrap()).sum();

        Ok(sum)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_claim<'a>(
        conn: &mut PgConnection,
        stake_addr: &'a String,
        payment_addr: &'a String,
        fingerprint: &'a String,
        amount: &'a u64,
        contract_id: &'a i64,
        user_id: &'a i64,
        txhash: &'a String,
        invalid: Option<&'a bool>,
        invalid_descr: Option<&'a String>,
    ) -> Result<Claimed, RWDError> {
        let new_claimed = ClaimedNew {
            stake_addr,
            payment_addr,
            fingerprint,
            amount: &BigDecimal::from_u64(*amount).unwrap(),
            contract_id,
            user_id,
            txhash,
            invalid,
            invalid_descr,
        };

        Ok(diesel::insert_into(claimed::table)
            .values(&new_claimed)
            .get_result::<Claimed>(conn)?)
    }

    pub fn invalidate_claim<'a>(
        conn: &mut PgConnection,
        stake_addr_in: &'a String,
        fingerprint_in: &'a String,
        contract_id_in: &'a i64,
        user_id_in: &'a i64,
        invalid_in: Option<&'a bool>,
        invalid_descr_in: Option<&'a String>,
    ) -> Result<Claimed, RWDError> {
        use crate::schema::claimed::dsl::*;
        let contract = diesel::update(
            claimed
                .filter(stake_addr.eq(stake_addr_in))
                .filter(fingerprint.eq(fingerprint_in))
                .filter(contract_id.eq(contract_id_in))
                .filter(user_id.eq(user_id_in)),
        )
        .set((invalid.eq(invalid_in), invalid_descr.eq(invalid_descr_in)))
        .get_result::<Claimed>(conn)?;

        Ok(contract)
    }

    pub fn get_stat_count_all_tx_on_contr(
        contract_id_in: i64,
        user_id_in: i64,
        fingerprint_in: Option<String>,
    ) -> Result<i64, RWDError> {
        use crate::schema::claimed::dsl::*;
        let mut gconn = establish_connection()?;

        let result = match fingerprint_in {
            Some(f) => {
                let result = claimed
                    .select(diesel::dsl::count_star())
                    .filter(fingerprint.eq(f))
                    .filter(contract_id.eq(contract_id_in))
                    .filter(user_id.eq(user_id_in))
                    .distinct_on(txhash)
                    .group_by((user_id, txhash, timestamp))
                    .load::<i64>(&mut gconn)?;

                result.len() as i64
            }
            None => {
                let result = claimed
                    .select(diesel::dsl::count_star())
                    .filter(contract_id.eq(contract_id_in))
                    .filter(user_id.eq(user_id_in))
                    .distinct_on(txhash)
                    .group_by((user_id, txhash, timestamp))
                    .load::<i64>(&mut gconn)?;

                result.len() as i64
            }
        };

        Ok(result)
    }

    pub fn get_stat_count_period_tx_token_user(
        fingerprint_in: &String,
        user_id_in: i64,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<i64, RWDError> {
        use crate::schema::claimed::dsl::*;
        let mut gconn = establish_connection()?;

        let result = claimed
            .select(diesel::dsl::count_star())
            .filter(fingerprint.eq(fingerprint_in))
            .filter(user_id.eq(user_id_in))
            .filter(timestamp.ge(from))
            .filter(timestamp.le(to))
            .distinct_on(txhash)
            .group_by((user_id, txhash, timestamp))
            .load::<i64>(&mut gconn)?;
        Ok(result.len() as i64)
    }

    pub fn get_stat_count_period_tx_contr_token(
        fingerprint_in: Option<String>,
        contract_id_in: i64,
        user_id_in: i64,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<i64, RWDError> {
        use crate::schema::claimed::dsl::*;
        let mut gconn = establish_connection()?;

        let result = match fingerprint_in {
            Some(f) => {
                let result = claimed
                    .select(diesel::dsl::count_star())
                    .filter(fingerprint.eq(f))
                    .filter(user_id.eq(user_id_in))
                    .filter(contract_id.eq(contract_id_in))
                    .filter(timestamp.ge(from))
                    .filter(timestamp.le(to))
                    .distinct_on(txhash)
                    .group_by((user_id, txhash, timestamp))
                    .load::<i64>(&mut gconn)?;

                result.len() as i64
            }
            None => {
                let result = claimed
                    .select(diesel::dsl::count_star())
                    .filter(user_id.eq(user_id_in))
                    .filter(contract_id.eq(contract_id_in))
                    .filter(timestamp.ge(from))
                    .filter(timestamp.le(to))
                    .distinct_on(txhash)
                    .group_by((user_id, txhash, timestamp))
                    .load::<i64>(&mut gconn)?;

                result.len() as i64
            }
        };
        Ok(result)
    }

    pub fn get_stat_count_all_tx_user(user_id_in: &i64) -> Result<i64, RWDError> {
        use crate::schema::claimed::dsl::*;
        let mut gconn = establish_connection()?;

        let result = claimed
            .select(diesel::dsl::count_star())
            .filter(user_id.eq(user_id_in))
            .distinct_on(txhash)
            .group_by((user_id, txhash, timestamp))
            .load::<i64>(&mut gconn)?;

        Ok(result.len() as i64)
    }

    pub fn get_stat_count_period_tx_user(
        user_id_in: &i64,
        from: &DateTime<Utc>,
        to: &DateTime<Utc>,
    ) -> Result<i64, RWDError> {
        use crate::schema::claimed::dsl::*;
        let mut gconn = establish_connection()?;

        let result = claimed
            .select(diesel::dsl::count_star())
            .filter(user_id.eq(user_id_in))
            .filter(timestamp.ge(from))
            .filter(timestamp.le(to))
            .distinct_on(txhash)
            .group_by((user_id, txhash, timestamp))
            .load::<i64>(&mut gconn)?;

        Ok(result.len() as i64)
    }
}

impl TokenWhitelist {
    pub fn get_whitelist() -> Result<Vec<TokenWhitelist>, RWDError> {
        let result = token_whitelist::table.load::<TokenWhitelist>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_epoch_filtered_whitelist(
        current_epoch: i64,
    ) -> Result<Vec<TokenWhitelist>, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let mut conn = establish_connection()?;
        let mut result = token_whitelist
            .filter(start_epoch.le(current_epoch))
            .load::<TokenWhitelist>(&mut conn)?;

        result.retain(|r| {
            if let Some(end) = r.end_epoch {
                end >= current_epoch && r.mode != Calculationmode::AirDrop
            } else {
                r.mode != Calculationmode::AirDrop
            }
        });

        Ok(result)
    }

    pub fn get_in_vesting_filtered_whitelist(
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<Vec<TokenWhitelist>, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let mut conn = establish_connection()?;
        let current_date = chrono::Utc::now();
        let result = token_whitelist
            .filter(contract_id.eq(contract_id_in))
            .filter(user_id.eq(user_id_in))
            .filter(vesting_period.gt(current_date))
            .load::<TokenWhitelist>(&mut conn)?;

        Ok(result)
    }

    pub fn has_contract_valid_whitelisting(
        contract_id_in: i64,
        user_id_in: i64,
        fingerprint_in: &String,
    ) -> Result<bool, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let mut conn = establish_connection()?;
        let current_date = chrono::Utc::now();
        let result = match token_whitelist
            .filter(contract_id.eq(contract_id_in))
            .filter(user_id.eq(user_id_in))
            .filter(fingerprint.eq(fingerprint_in))
            .filter(vesting_period.lt(current_date))
            .first::<TokenWhitelist>(&mut conn)
        {
            Ok(o) => {
                println!("O: {:?}", o);
                true
            }
            Err(e) => {
                println!("E: {:?}", e);
                false
            }
        };

        Ok(result)
    }

    pub fn get_token_info_ft(
        conn: &mut PgConnection,
        fingerprint_in: &String,
    ) -> Result<TokenInfo, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let result = token_whitelist
            .filter(fingerprint.is_not_null())
            .filter(fingerprint.eq(&fingerprint_in))
            .select((policy_id, tokenname.nullable(), fingerprint.nullable()))
            .first::<TokenInfo>(conn)?;
        Ok(result)
    }

    pub fn get_token_info_nft(
        conn: &mut PgConnection,
        fingerprint_in: &Option<String>,
        policy_id_in: String,
        tokenname_in: Option<String>,
    ) -> Result<TokenInfo, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let result: TokenInfo = match fingerprint_in {
            Some(fin) => token_whitelist
                .filter(fingerprint.is_not_null())
                .filter(fingerprint.eq(&fin))
                .select((policy_id, tokenname.nullable(), fingerprint.nullable()))
                .first::<TokenInfo>(conn)?,
            None => match tokenname_in {
                Some(tn) => token_whitelist
                    .filter(policy_id.eq(&policy_id_in))
                    .filter(tokenname.eq(&tn))
                    .select((policy_id, tokenname.nullable(), fingerprint.nullable()))
                    .first::<TokenInfo>(conn)?,
                None => {
                    return Err(RWDError::new(
                        "No tokenName and no fingerprint provided to retrieve tokeninfo",
                    ))
                }
            },
        };
        Ok(result)
    }

    pub fn get_rwd_contract_tokens(
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<Vec<TokenWhitelist>, RWDError> {
        use crate::schema::token_whitelist::dsl::*;

        let mut conn = establish_connection()?;
        let mut result = token_whitelist
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .load::<TokenWhitelist>(&mut conn)?;

        result.retain(|t| t.mode != Calculationmode::AirDrop);

        Ok(result)
    }

    pub fn get_user_tokens(user_id_in: &u64) -> Result<Vec<TokenWhitelist>, RWDError> {
        use crate::schema::token_whitelist::dsl::*;

        let mut conn = establish_connection()?;
        let mut result = token_whitelist
            .filter(user_id.eq(&(*user_id_in as i64)))
            .load::<TokenWhitelist>(&mut conn)?;

        result.retain(|t| t.mode != Calculationmode::AirDrop);

        Ok(result)
    }

    pub fn get_whitelist_entry(
        conn: &mut PgConnection,
        fingerprint_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<Vec<TokenWhitelist>, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let result = token_whitelist
            .filter(fingerprint.eq(&fingerprint_in))
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .load::<TokenWhitelist>(conn)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_twl_entry<'a>(
        conn: &mut PgConnection,
        fingerprint: &'a String,
        policy_id: &'a String,
        tokenname: &'a String,
        contract_id: &'a i64,
        user_id: &'a i64,
        vesting_period: &'a DateTime<Utc>,
        pools: &'a [GPools],
        mode: &'a Calculationmode,
        equation: &'a String,
        start_epoch_in: &'a i64,
        end_epoch: Option<&'a i64>,
        modificator_equ: Option<&'a String>,
    ) -> Result<TokenWhitelist, RWDError> {
        let mut spools = Vec::<String>::new();
        spools.extend(pools.iter().map(|n| n.to_string()));

        let new_twl_entry = TokenWhitelistNew {
            fingerprint,
            policy_id,
            tokenname,
            contract_id,
            user_id,
            vesting_period,
            pools: &spools,
            mode,
            equation,
            start_epoch: start_epoch_in,
            end_epoch,
            modificator_equ,
        };

        Ok(diesel::insert_into(token_whitelist::table)
            .values(&new_twl_entry)
            .get_result::<TokenWhitelist>(conn)?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_twl<'a>(
        conn: &mut PgConnection,
        fingerprint_in: &'a String,
        contract_id_in: &'a i64,
        user_id_in: &'a i64,
        vesting_period_in: &'a DateTime<Utc>,
        mode_in: &'a Calculationmode,
        equation_in: &'a String,
        start_epoch_in: &'a i64,
        end_epoch_in: Option<&'a i64>,
        modificator_equ_in: Option<&'a String>,
    ) -> Result<TokenWhitelist, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let twl = diesel::update(
            token_whitelist
                .filter(fingerprint.eq(fingerprint_in))
                .filter(contract_id.eq(contract_id_in))
                .filter(user_id.eq(user_id_in)),
        )
        .set((
            vesting_period.eq(vesting_period_in),
            mode.eq(mode_in),
            equation.eq(equation_in),
            start_epoch.eq(start_epoch_in),
            end_epoch.eq(end_epoch_in),
            modificator_equ.eq(modificator_equ_in),
        ))
        .get_result::<TokenWhitelist>(conn)?;

        Ok(twl)
    }

    pub fn remove_twl<'a>(
        fingerprint_in: &'a String,
        contract_id_in: &'a i64,
        user_id_in: &'a i64,
    ) -> Result<TokenWhitelist, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let mut conn = establish_connection()?;

        let result = diesel::delete(
            token_whitelist
                .filter(fingerprint.eq(fingerprint_in))
                .filter(contract_id.eq(contract_id_in))
                .filter(user_id.eq(user_id_in)),
        )
        .get_result::<TokenWhitelist>(&mut conn)?;

        Ok(result)
    }

    pub fn get_pools(
        fingerprint_in: &String,
        contract_id_in: &i64,
        user_id_in: &i64,
    ) -> Result<Vec<GPools>, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        use std::str::FromStr;
        let mut conn = establish_connection()?;
        let result = token_whitelist
            .filter(fingerprint.eq(&fingerprint_in))
            .filter(contract_id.eq(contract_id_in))
            .filter(user_id.eq(user_id_in))
            .select(pools)
            .first::<Vec<String>>(&mut conn)?;
        let mut resp = Vec::<GPools>::new();
        resp.extend(
            result
                .iter()
                .map(|n| GPools::from_str(n).expect("Could not convert string to GPools")),
        );

        Ok(resp)
    }

    pub fn add_pools<'a>(
        fingerprint_in: &'a String,
        contract_id_in: &'a i64,
        user_id_in: &'a i64,
        pools_in: &'a [GPools],
    ) -> Result<TokenWhitelist, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        use itertools::Itertools;
        use std::str::FromStr;

        let mut conn = establish_connection()?;
        let twl = TokenWhitelist::get_whitelist_entry(
            &mut conn,
            fingerprint_in,
            *contract_id_in,
            *user_id_in,
        )?;

        let mut old_pools = Vec::<GPools>::new();
        old_pools.extend(
            twl[0]
                .pools
                .iter()
                .map(|n| GPools::from_str(n).expect("Could not convert string to GPools")),
        );
        old_pools.extend(pools_in.iter().cloned());
        let npool: Vec<_> = old_pools.iter().unique_by(|p| p.pool_id.clone()).collect();

        let mut spools = Vec::<String>::new();
        spools.extend(npool.iter().map(|n| n.to_string()));

        let result = diesel::update(
            token_whitelist
                .filter(fingerprint.eq(fingerprint_in))
                .filter(contract_id.eq(contract_id_in))
                .filter(user_id.eq(user_id_in)),
        )
        .set(pools.eq(spools))
        .get_result::<TokenWhitelist>(&mut conn)?;

        Ok(result)
    }

    pub fn remove_pools<'a>(
        fingerprint_in: &'a String,
        contract_id_in: &'a i64,
        user_id_in: &'a i64,
        pools_in: &'a [GPools],
    ) -> Result<TokenWhitelist, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        use std::str::FromStr;

        let mut conn = establish_connection()?;
        let twl = TokenWhitelist::get_whitelist_entry(
            &mut conn,
            fingerprint_in,
            *contract_id_in,
            *user_id_in,
        )?;

        let mut old_pools = Vec::<GPools>::new();
        old_pools.extend(
            twl[0]
                .pools
                .iter()
                .map(|n| GPools::from_str(n).expect("Could not convert string to GPools")),
        );
        old_pools.retain(|p| !pools_in.contains(p));

        let mut spools = Vec::<String>::new();
        spools.extend(old_pools.iter().map(|n| n.to_string()));

        let result = diesel::update(
            token_whitelist
                .filter(fingerprint.eq(fingerprint_in))
                .filter(contract_id.eq(contract_id_in))
                .filter(user_id.eq(user_id_in)),
        )
        .set(pools.eq(spools.clone()))
        .get_result::<TokenWhitelist>(&mut conn)?;

        Ok(result)
    }
}

impl AirDropWhitelist {
    pub fn get_all_whitelist_entrys(
        conn: &mut PgConnection,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<Vec<AirDropWhitelist>, RWDError> {
        use crate::schema::airdrop_whitelist::dsl::*;
        let result = airdrop_whitelist
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .load::<AirDropWhitelist>(conn)?;
        Ok(result)
    }

    pub fn create_awl_entry<'a>(
        conn: &mut PgConnection,
        contract_id: &'a i64,
        user_id: &'a i64,
    ) -> Result<AirDropWhitelist, RWDError> {
        let new_twl_entry = AirDropWhitelistNew {
            contract_id,
            user_id,
            reward_created: &false,
        };

        Ok(diesel::insert_into(airdrop_whitelist::table)
            .values(&new_twl_entry)
            .get_result::<AirDropWhitelist>(conn)?)
    }

    pub fn use_awl<'a>(
        conn: &mut PgConnection,
        contract_id_in: &'a i64,
        user_id_in: &'a i64,
    ) -> Result<AirDropWhitelist, RWDError> {
        use crate::schema::airdrop_whitelist::dsl::*;
        let twl = diesel::update(
            airdrop_whitelist
                .filter(contract_id.eq(contract_id_in))
                .filter(user_id.eq(user_id_in)),
        )
        .set(reward_created.eq(true))
        .get_result::<AirDropWhitelist>(conn)?;

        Ok(twl)
    }

    pub fn remove_awl<'a>(
        conn: &mut PgConnection,
        contract_id_in: &'a i64,
        user_id_in: &'a i64,
    ) -> Result<usize, RWDError> {
        use crate::schema::airdrop_whitelist::dsl::*;

        let result = diesel::delete(
            airdrop_whitelist
                .filter(contract_id.eq(contract_id_in))
                .filter(user_id.eq(user_id_in)),
        )
        .filter(reward_created.eq(true))
        .execute(conn)?;

        Ok(result)
    }
}

impl AirDropParameter {
    pub fn get_ad_parameters_for_contract(
        conn: &mut PgConnection,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<Vec<AirDropParameter>, RWDError> {
        use crate::schema::airdrop_parameter::dsl::*;
        let result = airdrop_parameter
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .load::<AirDropParameter>(conn)?;
        Ok(result)
    }

    pub fn get_ad_parameter(
        conn: &mut PgConnection,
        id_in: i64,
    ) -> Result<AirDropParameter, RWDError> {
        use crate::schema::airdrop_parameter::dsl::*;
        let result = airdrop_parameter
            .find(id_in)
            .first::<AirDropParameter>(conn)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_airdrop_parameter<'a>(
        conn: &mut PgConnection,
        contract_id: &'a i64,
        user_id: &'a i64,
        airdrop_token_type: &'a String,
        distribution_type: &'a String,
        selection_type: &'a String,
        args_1: &'a Vec<String>,
        args_2: &'a Vec<String>,
        args_3: &'a Vec<String>,
        whitelist_ids: Option<&'a Vec<i64>>,
    ) -> Result<AirDropParameter, RWDError> {
        let new_adp_entry = AirDropParameterNew {
            contract_id,
            user_id,
            airdrop_token_type,
            distribution_type,
            selection_type,
            args_1,
            args_2,
            args_3,
            whitelist_ids,
        };

        Ok(diesel::insert_into(airdrop_parameter::table)
            .values(&new_adp_entry)
            .get_result::<AirDropParameter>(conn)?)
    }

    pub fn remove_airdrop_parameter(
        conn: &mut PgConnection,
        id_in: i64,
    ) -> Result<usize, RWDError> {
        use crate::schema::airdrop_parameter::dsl::*;
        let result = diesel::delete(airdrop_parameter.find(id_in)).execute(conn)?;

        Ok(result)
    }
}

impl WlAddresses {
    pub fn get_wladdress(conn: &mut PgConnection, id_in: i64) -> Result<WlAddresses, RWDError> {
        use crate::schema::wladdresses::dsl::*;
        let result = wladdresses.find(id_in).first::<WlAddresses>(conn)?;
        Ok(result)
    }

    pub fn get_address(addr: String) -> Result<Vec<WlAddresses>, RWDError> {
        use crate::schema::wladdresses::dsl::*;
        let conn = &mut establish_connection()?;
        let result = wladdresses
            .filter(payment_address.eq(addr))
            .load::<WlAddresses>(conn)?;
        Ok(result)
    }

    pub fn get_stake_address(addr: String) -> Result<Vec<WlAddresses>, RWDError> {
        use crate::schema::wladdresses::dsl::*;
        let conn = &mut establish_connection()?;
        let result = wladdresses
            .filter(stake_address.eq(addr).nullable())
            .load::<WlAddresses>(conn)?;
        Ok(result)
    }

    pub fn create_wladdress(
        conn: &mut PgConnection,
        address: &String,
        stake_address: Option<&String>,
    ) -> Result<WlAddresses, RWDError> {
        let new_entry = WlAddressesNew {
            payment_address: address,
            stake_address,
        };

        Ok(diesel::insert_into(wladdresses::table)
            .values(&new_entry)
            .on_conflict_do_nothing()
            .get_result::<WlAddresses>(conn)?)
    }

    pub fn remove_wladdress(conn: &mut PgConnection, id_in: i64) -> Result<usize, RWDError> {
        use crate::schema::wladdresses::dsl::*;
        let result = diesel::delete(wladdresses.find(id_in)).execute(conn)?;

        Ok(result)
    }
}

impl WlAlloc {
    pub fn get_whitelist(id_in: &i64) -> Result<Vec<String>, RWDError> {
        let result = wlalloc::table
            .inner_join(wladdresses::table.on(wlalloc::addr.eq(wladdresses::id)))
            .filter(wlalloc::wl.eq(id_in))
            .select(wladdresses::payment_address)
            .load::<String>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_whitelist_entries(
        user_id_in: &i64,
        whitelist_id_in: &i64,
    ) -> Result<Vec<WlEntry>, RWDError> {
        Whitelist::get_whitelist(user_id_in, whitelist_id_in)?;
        let result = wlalloc::table
            .inner_join(wladdresses::table.on(wlalloc::addr.eq(wladdresses::id)))
            .filter(wlalloc::wl.eq(whitelist_id_in))
            .select((
                wladdresses::id,
                wladdresses::payment_address,
                wladdresses::stake_address,
                wlalloc::wl,
                wlalloc::addr,
                wlalloc::specific_asset,
            ))
            .load::<WlEntry>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_address_whitelist(id_in: &i64, payaddr: &String) -> Result<Vec<WlEntry>, RWDError> {
        let result = wlalloc::table
            .inner_join(wladdresses::table.on(wlalloc::addr.eq(wladdresses::id)))
            .filter(wladdresses::payment_address.eq(payaddr))
            .filter(wlalloc::wl.eq(id_in))
            .select((
                wladdresses::id,
                wladdresses::payment_address,
                wladdresses::stake_address,
                wlalloc::wl,
                wlalloc::addr,
                wlalloc::specific_asset,
            ))
            .load::<WlEntry>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_address_allocations(payaddr: &String) -> Result<Vec<WlEntry>, RWDError> {
        let result = wlalloc::table
            .inner_join(wladdresses::table.on(wlalloc::addr.eq(wladdresses::id)))
            .filter(wladdresses::payment_address.eq(payaddr))
            .select((
                wladdresses::id,
                wladdresses::payment_address,
                wladdresses::stake_address,
                wlalloc::wl,
                wlalloc::addr,
                wlalloc::specific_asset,
            ))
            .load::<WlEntry>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_specific_address_allocations(
        payaddr: &String,
        asset: &serde_json::Value,
    ) -> Result<WlEntry, RWDError> {
        let result = wlalloc::table
            .inner_join(wladdresses::table.on(wlalloc::addr.eq(wladdresses::id)))
            .filter(wladdresses::payment_address.eq(payaddr))
            .filter(wlalloc::specific_asset.eq(asset).nullable())
            .select((
                wladdresses::id,
                wladdresses::payment_address,
                wladdresses::stake_address,
                wlalloc::wl,
                wlalloc::addr,
                wlalloc::specific_asset,
            ))
            .first::<WlEntry>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn check_stake_address_in_whitelist(
        id_in: &i64,
        stake_address: &String,
    ) -> Result<Vec<WlEntry>, RWDError> {
        let result = wlalloc::table
            .inner_join(wladdresses::table.on(wlalloc::addr.eq(wladdresses::id)))
            .filter(wlalloc::wl.eq(id_in))
            .filter(wladdresses::stake_address.eq(stake_address).nullable())
            .select((
                wladdresses::id,
                wladdresses::payment_address,
                wladdresses::stake_address,
                wlalloc::wl,
                wlalloc::addr,
                wlalloc::specific_asset,
            ))
            .load::<WlEntry>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn check_pay_address_in_whitelist(
        id_in: &i64,
        address: &String,
    ) -> Result<Vec<WlEntry>, RWDError> {
        let result = wlalloc::table
            .inner_join(wladdresses::table.on(wlalloc::addr.eq(wladdresses::id)))
            .filter(wlalloc::wl.eq(id_in))
            .filter(wladdresses::payment_address.eq(address).nullable())
            .select((
                wladdresses::id,
                wladdresses::payment_address,
                wladdresses::stake_address,
                wlalloc::wl,
                wlalloc::addr,
                wlalloc::specific_asset,
            ))
            .load::<WlEntry>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn create_alloc<'a>(
        wl_in: &'a i64,
        addr_in: &'a i64,
        specific_asset: Option<&'a SpecificAsset>,
    ) -> Result<WlAlloc, RWDError> {
        let sa = if let Some(s) = specific_asset {
            Some(serde_json::to_value(s)?)
        } else {
            None
        };

        let new_entry = WlAllocNew {
            wl: wl_in,
            addr: addr_in,
            specific_asset: sa.as_ref(),
        };
        match diesel::insert_into(wlalloc::table)
            .values(&new_entry)
            .get_result::<WlAlloc>(&mut establish_connection()?)
        {
            Ok(o) => Ok(o),
            Err(_) => Ok(diesel::update(
                wlalloc::table
                    .filter(wlalloc::addr.eq(addr_in))
                    .filter(wlalloc::wl.eq(wl_in)),
            )
            .set(wlalloc::specific_asset.eq(serde_json::json!(specific_asset)))
            .get_result::<WlAlloc>(&mut establish_connection()?)?),
        }
    }

    pub fn update_alloc<'a>(
        wl_in: &'a i64,
        addr_in: &'a i64,
        specific_asset_in: Option<&'a SpecificAsset>,
    ) -> Result<WlAlloc, RWDError> {
        let sa = if let Some(s) = specific_asset_in {
            Some(serde_json::to_value(s)?)
        } else {
            None
        };

        Ok(diesel::update(
            wlalloc::table
                .filter(wlalloc::wl.eq(wl_in))
                .filter(wlalloc::addr.eq(addr_in)),
        )
        .set(wlalloc::specific_asset.eq(sa))
        .get_result::<WlAlloc>(&mut establish_connection()?)?)
    }

    pub fn remove_wlentry<'a>(wl_in: &'a i64, addr_in: &'a i64) -> Result<usize, RWDError> {
        let result = diesel::delete(wlalloc::table.find((wl_in, addr_in)))
            .execute(&mut establish_connection()?)?;

        Ok(result)
    }

    pub fn remove_wl(wl_in: &i64) -> Result<usize, RWDError> {
        let result = diesel::delete(wlalloc::table.filter(wlalloc::wl.eq(wl_in)))
            .execute(&mut establish_connection()?)?;
        Ok(result)
    }
}

impl Whitelist {
    pub fn get_whitelist(user_id_in: &i64, id_in: &i64) -> Result<Whitelist, RWDError> {
        let result = whitelist::table
            .filter(whitelist::id.eq(id_in))
            .filter(whitelist::user_id.eq(user_id_in))
            .first::<Whitelist>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_whitelists(user_id_in: &i64) -> Result<Whitelist, RWDError> {
        let result = whitelist::table
            .filter(whitelist::user_id.eq(user_id_in))
            .first::<Whitelist>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn create_whitelist(
        user_id_in: &i64,
        max_addr_repeat: &i32,
        wl_type: &WhitelistType,
        description: &String,
        notes: &String,
    ) -> Result<Whitelist, RWDError> {
        let new_entry = WhitelistNew {
            user_id: user_id_in,
            max_addr_repeat,
            wl_type,
            description,
            notes,
        };

        Ok(diesel::insert_into(whitelist::table)
            .values(&new_entry)
            .get_result::<Whitelist>(&mut establish_connection()?)?)
    }

    pub fn add_to_wl<'a>(
        wl_in: &'a i64,
        address: &'a String,
        stake_address: Option<&'a String>,
        specific_asset: Option<&'a SpecificAsset>,
    ) -> Result<WlAlloc, RWDError> {
        let raddr = match WlAddresses::create_wladdress(
            &mut establish_connection()?,
            address,
            stake_address,
        ) {
            Ok(o) => o,
            Err(e) => {
                log::error!("Error creating new wladdress: {:?}", e.to_string());
                let addr = WlAddresses::get_address(address.to_string())?;
                if addr.is_empty() {
                    return Err(RWDError::new("Could also not find address"));
                }
                addr[0].clone()
            }
        };
        let ralloc = WlAlloc::create_alloc(wl_in, &raddr.id, specific_asset)?;
        Ok(ralloc)
    }

    pub fn remove_from_wl<'a>(
        user_id: &'a i64,
        wl_id: &'a i64,
        address: &'a String,
    ) -> Result<(), RWDError> {
        Whitelist::get_whitelist(user_id, wl_id)?;

        let allocs = WlAlloc::get_address_whitelist(wl_id, address)?;

        for a in &allocs {
            WlAlloc::remove_wlentry(&a.wl, &a.alloc_id)?;
        }
        let addr_id = allocs[0].alloc_id;
        let allocs = WlAlloc::get_address_allocations(address)?;

        if allocs.is_empty() {
            WlAddresses::remove_wladdress(&mut establish_connection()?, addr_id)?;
        }
        Ok(())
    }

    pub fn remove_wl(user_id_in: &i64, wl_in: &i64) -> Result<usize, RWDError> {
        Whitelist::get_whitelist(user_id_in, wl_in)?;
        WlAlloc::remove_wl(wl_in)?;
        let result = diesel::delete(whitelist::table.filter(whitelist::id.eq(wl_in)))
            .execute(&mut establish_connection()?)?;
        Ok(result)
    }
}

impl Discount {
    pub fn get_discounts(cid_in: i64, uid_in: i64) -> Result<Vec<Discount>, RWDError> {
        let conn = &mut establish_connection()?;
        let result = discount::table
            .filter(discount::contract_id.eq(cid_in))
            .filter(discount::user_id.eq(uid_in))
            .load::<Discount>(conn)?;
        Ok(result)
    }

    pub fn create_discount(
        user_id: &i64,
        contract_id: &i64,
        policy_id: &String,
        fingerprint: Option<&String>,
        metadata_path: &Vec<String>,
    ) -> Result<Discount, RWDError> {
        let conn = &mut establish_connection()?;
        let new_entry = DiscountNew {
            contract_id,
            user_id,
            policy_id,
            fingerprint,
            metadata_path,
        };

        Ok(diesel::insert_into(discount::table)
            .values(&new_entry)
            .get_result::<Discount>(conn)?)
    }

    pub fn remove_discount(id: &i64) -> Result<usize, RWDError> {
        let conn = &mut establish_connection()?;

        let result = diesel::delete(discount::table.filter(discount::id.eq(id))).execute(conn)?;
        Ok(result)
    }

    pub fn policy_id(&self) -> String {
        self.policy_id.clone()
    }

    pub fn fingerprint(&self) -> Option<&String> {
        self.fingerprint.as_ref()
    }
}
