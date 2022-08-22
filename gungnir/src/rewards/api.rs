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

impl Rewards {
    pub fn get_rewards_stake_addr(
        conn: &PgConnection,
        stake_addr_in: String,
    ) -> Result<Vec<Rewards>, RWDError> {
        use crate::schema::rewards::dsl::*;
        let result = rewards
            .filter(stake_addr.eq(&stake_addr_in))
            .load::<Rewards>(conn)?;
        Ok(result)
    }

    pub fn get_rewards(
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
                    if cs < r.tot_earned.to_i64().unwrap() {
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
        conn: &PgConnection,
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
            .load::<Rewards>(conn)?;
        Ok(result)
    }

    pub fn get_available_rewards(
        conn: &PgConnection,
        stake_addr_in: &String,
        payment_addr_in: &String,
        fingerprint_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
        claim_request: i64,
    ) -> Result<i64, RWDError> {
        use crate::schema::rewards::dsl::*;
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
        let claim_sum = Claimed::get_token_claims_tot_amt(
            conn,
            stake_addr_in,
            fingerprint_in,
            contract_id_in,
            user_id_in,
        )?;
        log::info!("found claims");
        let lovelace = BigDecimal::from_i32(1000000).unwrap();
        match ((result.0 / lovelace) - result.1.clone()).to_i64() {
            Some(dif) => {
                if claim_sum != result.1.to_i64().unwrap() {
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
        stake_addr_in: &String,
        fingerprint_in: &String,
        contract_id_in: i64,
        user_id_in: i64,
    ) -> Result<i64, RWDError> {
        use crate::schema::claimed::dsl::*;
        let result = claimed
            .filter(stake_addr.eq(stake_addr_in))
            .filter(contract_id.eq(contract_id_in))
            .filter(user_id.eq(user_id_in))
            .filter(fingerprint.eq(fingerprint_in))
            .filter(invalid.is_null())
            .select(amount)
            .load::<BigDecimal>(conn)?;

        let sum = result.iter().map(|x| x.to_i64().unwrap()).sum();

        Ok(sum)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_claim<'a>(
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        let gconn = establish_connection()?;

        let result = match fingerprint_in {
            Some(f) => {
                let result = claimed
                    .select(diesel::dsl::count_star())
                    .filter(fingerprint.eq(f))
                    .filter(contract_id.eq(contract_id_in))
                    .filter(user_id.eq(user_id_in))
                    .distinct_on(txhash)
                    .group_by((user_id, txhash, timestamp))
                    //.count()
                    .load::<i64>(&gconn)?; //(i64,String,DateTime<Utc>)

                result.len() as i64
            }
            None => {
                let result = claimed
                    .select(diesel::dsl::count_star())
                    .filter(contract_id.eq(contract_id_in))
                    .filter(user_id.eq(user_id_in))
                    .distinct_on(txhash)
                    .group_by((user_id, txhash, timestamp))
                    //.count()
                    .load::<i64>(&gconn)?; //(i64,String,DateTime<Utc>)

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
        let gconn = establish_connection()?;

        let result = claimed
            .select(diesel::dsl::count_star())
            .filter(fingerprint.eq(fingerprint_in))
            .filter(user_id.eq(user_id_in))
            .filter(timestamp.ge(from))
            .filter(timestamp.le(to))
            .distinct_on(txhash)
            .group_by((user_id, txhash, timestamp))
            //.count()
            .load::<i64>(&gconn)?; //(i64,String,DateTime<Utc>)
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
        let gconn = establish_connection()?;

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
                    //.count()
                    .load::<i64>(&gconn)?; //(i64,String,DateTime<Utc>)

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
                    .load::<i64>(&gconn)?;

                result.len() as i64
            }
        };
        Ok(result)
    }

    pub fn get_stat_count_all_tx_user(user_id_in: &i64) -> Result<i64, RWDError> {
        use crate::schema::claimed::dsl::*;
        let gconn = establish_connection()?;

        let result = claimed
            .select((user_id, txhash))
            .distinct_on(txhash)
            .filter(user_id.eq(user_id_in))
            .count()
            .first::<i64>(&gconn);

        Ok(result?)
    }

    pub fn get_stat_count_period_tx_user(
        user_id_in: &i64,
        from: &DateTime<Utc>,
        to: &DateTime<Utc>,
    ) -> Result<i64, RWDError> {
        use crate::schema::claimed::dsl::*;
        let gconn = establish_connection()?;

        let result = claimed
            .select(diesel::dsl::count_star())
            .filter(user_id.eq(user_id_in))
            .filter(timestamp.ge(from))
            .filter(timestamp.le(to))
            .distinct_on(txhash)
            .group_by((user_id, txhash, timestamp))
            .load::<i64>(&gconn)?;

        Ok(result.len() as i64)
    }
}

impl TokenWhitelist {
    pub fn get_whitelist(conn: &PgConnection) -> Result<Vec<TokenWhitelist>, RWDError> {
        let result = token_whitelist::table.load::<TokenWhitelist>(conn)?;
        Ok(result)
    }

    pub fn get_epoch_filtered_whitelist(
        current_epoch: i64,
    ) -> Result<Vec<TokenWhitelist>, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let conn = establish_connection()?;
        let mut result = token_whitelist
            .filter(start_epoch.le(current_epoch))
            .load::<TokenWhitelist>(&conn)?;

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
        let conn = establish_connection()?;
        let current_date = chrono::Utc::now();
        let result = token_whitelist
            .filter(contract_id.eq(contract_id_in))
            .filter(user_id.eq(user_id_in))
            .filter(vesting_period.gt(current_date))
            .load::<TokenWhitelist>(&conn)?;

        Ok(result)
    }

    pub fn has_contract_valid_whitelisting(
        contract_id_in: i64,
        user_id_in: i64,
        fingerprint_in: &String,
    ) -> Result<bool, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        let conn = establish_connection()?;
        let current_date = chrono::Utc::now();
        let result = match token_whitelist
            .filter(contract_id.eq(contract_id_in))
            .filter(user_id.eq(user_id_in))
            .filter(fingerprint.eq(fingerprint_in))
            .filter(vesting_period.lt(current_date))
            .first::<TokenWhitelist>(&conn)
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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

        let conn = establish_connection()?;
        let mut result = token_whitelist
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .load::<TokenWhitelist>(&conn)?;

        result.retain(|t| t.mode != Calculationmode::AirDrop);

        Ok(result)
    }

    pub fn get_whitelist_entry(
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        let conn = establish_connection()?;

        let result = diesel::delete(
            token_whitelist
                .filter(fingerprint.eq(fingerprint_in))
                .filter(contract_id.eq(contract_id_in))
                .filter(user_id.eq(user_id_in)),
        )
        .get_result::<TokenWhitelist>(&conn)?;

        Ok(result)
    }

    pub fn get_pools(
        fingerprint_in: &String,
        contract_id_in: &i64,
        user_id_in: &i64,
    ) -> Result<Vec<GPools>, RWDError> {
        use crate::schema::token_whitelist::dsl::*;
        use std::str::FromStr;
        let conn = establish_connection()?;
        let result = token_whitelist
            .filter(fingerprint.eq(&fingerprint_in))
            .filter(contract_id.eq(contract_id_in))
            .filter(user_id.eq(user_id_in))
            .select(pools)
            .first::<Vec<String>>(&conn)?;
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

        let conn = establish_connection()?;
        let twl = TokenWhitelist::get_whitelist_entry(
            &conn,
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
        .get_result::<TokenWhitelist>(&conn)?;

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

        let conn = establish_connection()?;
        let twl = TokenWhitelist::get_whitelist_entry(
            &conn,
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
        .get_result::<TokenWhitelist>(&conn)?;

        Ok(result)
    }
}

impl AirDropWhitelist {
    pub fn get_all_whitelist_entrys(
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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
        conn: &PgConnection,
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

    pub fn get_ad_parameter(conn: &PgConnection, id_in: i64) -> Result<AirDropParameter, RWDError> {
        use crate::schema::airdrop_parameter::dsl::*;
        let result = airdrop_parameter
            .find(id_in)
            .first::<AirDropParameter>(conn)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_airdrop_parameter<'a>(
        conn: &PgConnection,
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

    pub fn remove_airdrop_parameter(conn: &PgConnection, id_in: i64) -> Result<usize, RWDError> {
        use crate::schema::airdrop_parameter::dsl::*;
        let result = diesel::delete(airdrop_parameter.find(id_in)).execute(conn)?;

        Ok(result)
    }
}

impl WlAddresses {
    pub fn get_wladdress(conn: &PgConnection, id_in: i64) -> Result<WlAddresses, RWDError> {
        use crate::schema::wladdresses::dsl::*;
        let result = wladdresses.find(id_in).first::<WlAddresses>(conn)?;
        Ok(result)
    }

    pub fn create_wladdress(
        conn: &PgConnection,
        address: &String,
    ) -> Result<WlAddresses, RWDError> {
        let new_entry = WlAddressesNew {
            payment_address: address,
        };

        Ok(diesel::insert_into(wladdresses::table)
            .values(&new_entry)
            .on_conflict_do_nothing()
            .get_result::<WlAddresses>(conn)?)
    }

    pub fn remove_wladdress(conn: &PgConnection, id_in: i64) -> Result<usize, RWDError> {
        use crate::schema::wladdresses::dsl::*;
        let result = diesel::delete(wladdresses.find(id_in)).execute(conn)?;

        Ok(result)
    }
}

impl WlAlloc {
    pub fn get_whitelist(conn: &PgConnection, id_in: i64) -> Result<Vec<String>, RWDError> {
        let result = wlalloc::table
            .inner_join(wladdresses::table.on(wlalloc::addr.eq(wladdresses::id)))
            .filter(wlalloc::wl.eq(id_in))
            .select(wladdresses::payment_address)
            .load::<String>(conn)?;
        Ok(result)
    }

    pub fn create_alloc<'a>(
        conn: &PgConnection,
        wl_in: &'a i64,
        addr_in: &'a i64,
    ) -> Result<WlAlloc, RWDError> {
        let new_entry = WlAllocNew {
            wl: wl_in,
            addr: addr_in,
        };

        Ok(diesel::insert_into(wlalloc::table)
            .values(&new_entry)
            .get_result::<WlAlloc>(conn)?)
    }

    pub fn remove_wlentry<'a>(
        conn: &PgConnection,
        wl_in: &'a i64,
        addr_in: &'a i64,
    ) -> Result<usize, RWDError> {
        let result = diesel::delete(wlalloc::table.find((wl_in, addr_in))).execute(conn)?;

        Ok(result)
    }

    pub fn remove_wl(conn: &PgConnection, wl_in: &i64) -> Result<usize, RWDError> {
        let result = diesel::delete(wlalloc::table.filter(wlalloc::wl.eq(wl_in))).execute(conn)?;
        Ok(result)
    }
}

impl Whitelist {
    pub fn get_whitelist(conn: &PgConnection, id_in: i64) -> Result<Whitelist, RWDError> {
        let result = whitelist::table
            .filter(whitelist::id.eq(id_in))
            .first::<Whitelist>(conn)?;
        Ok(result)
    }

    pub fn create_wladdress(
        conn: &PgConnection,
        max_addr_repeat: &i32,
    ) -> Result<Whitelist, RWDError> {
        let new_entry = WhitelistNew { max_addr_repeat };

        Ok(diesel::insert_into(whitelist::table)
            .values(&new_entry)
            .get_result::<Whitelist>(conn)?)
    }

    pub fn add_to_wl<'a>(
        conn: &PgConnection,
        wl_in: &'a i64,
        address: &'a String,
    ) -> Result<WlAlloc, RWDError> {
        let raddr = WlAddresses::create_wladdress(conn, address)?;
        let ralloc = WlAlloc::create_alloc(conn, wl_in, &raddr.id)?;
        Ok(ralloc)
    }

    pub fn remove_wl(conn: &PgConnection, wl_in: &i64) -> Result<usize, RWDError> {
        let _result = WlAlloc::remove_wl(conn, wl_in)?;

        let result =
            diesel::delete(whitelist::table.filter(whitelist::id.eq(wl_in))).execute(conn)?;
        Ok(result)
    }
}

impl MintProject {
    pub fn get_mintproject_by_id(conn: &PgConnection, id_in: i64) -> Result<MintProject, RWDError> {
        let result = mint_projects::table
            .filter(mint_projects::id.eq(id_in))
            .first::<MintProject>(conn)?;
        Ok(result)
    }

    pub fn get_mintproject_by_uid_cid(
        conn: &PgConnection,
        uid_in: i64,
        cid_in: i64,
    ) -> Result<MintProject, RWDError> {
        let result = mint_projects::table
            .filter(mint_projects::user_id.eq(uid_in))
            .filter(mint_projects::contract_id.eq(cid_in))
            .first::<MintProject>(conn)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_mintproject<'a>(
        conn: &PgConnection,
        customer_name: &'a String,
        project_name: &'a String,
        user_id: &'a i64,
        contract_id: &'a i64,
        whitelist_id: Option<&'a i64>,
        mint_start_date: &'a DateTime<Utc>,
        mint_end_date: Option<&'a DateTime<Utc>>,
        storage_folder: &'a String,
        max_trait_count: &'a i32,
        collection_name: &'a String,
        author: &'a String,
        meta_description: &'a String,
        max_mint_p_addr: Option<&'a i32>,
        reward_minter: &'a bool,
    ) -> Result<MintProject, RWDError> {
        let new_entry = MintProjectNew {
            customer_name,
            project_name,
            user_id,
            contract_id,
            whitelist_id,
            mint_start_date,
            mint_end_date,
            storage_folder,
            max_trait_count,
            collection_name,
            author,
            meta_description,
            max_mint_p_addr,
            reward_minter,
        };

        Ok(diesel::insert_into(mint_projects::table)
            .values(&new_entry)
            .get_result::<MintProject>(conn)?)
    }

    pub fn remove_mintproject(conn: &PgConnection, id_in: &i64) -> Result<usize, RWDError> {
        let result = diesel::delete(mint_projects::table.find(id_in)).execute(conn)?;

        Ok(result)
    }
}

impl Nft {
    pub fn get_nfts_by_pid(conn: &PgConnection, id_in: i64) -> Result<Vec<Nft>, RWDError> {
        let result = nft_table::table
            .filter(nft_table::project_id.eq(id_in))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_nft_by_assetnameb(
        conn: &PgConnection,
        pid_in: i64,
        assetname_in: Vec<u8>,
    ) -> Result<Nft, RWDError> {
        let result = nft_table::table
            .find((pid_in, assetname_in))
            .first::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_nft_by_assetname_str(
        conn: &PgConnection,
        pid_in: i64,
        assetname_in: &String,
    ) -> Result<Nft, RWDError> {
        let result = nft_table::table
            .filter(nft_table::project_id.eq(pid_in))
            .filter(nft_table::asset_name.eq(assetname_in))
            .first::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_random_unminted_nft(
        conn: &PgConnection,
        pid_in: i64,
    ) -> Result<Option<Nft>, RWDError> {
        use rand::{thread_rng, Rng};

        let result = nft_table::table
            .filter(nft_table::project_id.eq(pid_in))
            .filter(nft_table::minted.eq(false))
            .filter(nft_table::payment_addr.is_null())
            .filter(nft_table::tx_hash.is_null())
            .load::<Nft>(conn)?;

        let mut rng = thread_rng();
        let rnd: usize = rng.gen_range(0..=result.len());
        let nft = result.get(rnd);
        Ok(nft.cloned())
    }

    pub fn get_nft_by_payaddr(
        conn: &PgConnection,
        pid_in: i64,
        payment_addr: &String,
    ) -> Result<Vec<Nft>, RWDError> {
        let result = nft_table::table
            .filter(nft_table::project_id.eq(pid_in))
            .filter(nft_table::payment_addr.eq(payment_addr))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_nft_by_payaddr_unminted(
        conn: &PgConnection,
        pid_in: i64,
        payment_addr: &String,
    ) -> Result<Vec<Nft>, RWDError> {
        let result = nft_table::table
            .filter(nft_table::project_id.eq(pid_in))
            .filter(nft_table::payment_addr.eq(payment_addr))
            .filter(nft_table::minted.eq(false))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_nft<'a>(
        conn: &PgConnection,
        project_id: &'a i64,
        assetname_b: &'a Vec<u8>,
        asset_name: &'a String,
        picture_id: &'a String,
        file_name: &'a String,
        ipfs_hash: Option<&'a String>,
        trait_category: &'a Vec<String>,
        traits: &'a Vec<Vec<String>>,
        metadata: &'a String,
        payment_addr: Option<&'a String>,
        minted: &'a bool,
        tx_hash: Option<&'a String>,
        confirmed: &'a bool,
    ) -> Result<Nft, RWDError> {
        let new_entry = NftNew {
            project_id,
            asset_name_b: assetname_b,
            asset_name,
            picture_id,
            file_name,
            ipfs_hash,
            trait_category,
            traits,
            metadata,
            payment_addr,
            minted,
            tx_hash,
            confirmed,
        };

        Ok(diesel::insert_into(nft_table::table)
            .values(&new_entry)
            .get_result::<Nft>(conn)?)
    }

    pub fn set_nft_minted<'a>(
        conn: &PgConnection,
        id_in: &'a i64,
        assetnameb: &'a Vec<u8>,
        txhash_in: &'a String,
    ) -> Result<Nft, RWDError> {
        let result = diesel::update(nft_table::table.find((id_in, assetnameb)))
            .set((
                nft_table::minted.eq(true),
                nft_table::tx_hash.eq(Some(txhash_in)),
            ))
            .get_result::<Nft>(conn)?;

        Ok(result)
    }

    pub fn set_nft_confirmed<'a>(
        conn: &PgConnection,
        id_in: &'a i64,
        assetnameb: &'a Vec<u8>,
        txhash_in: &'a String,
    ) -> Result<Nft, RWDError> {
        let nft = Nft::get_nft_by_assetnameb(conn, *id_in, assetnameb.clone())?;
        if let Some(hash) = nft.tx_hash {
            if hash == *txhash_in {
                let result = diesel::update(nft_table::table.find((id_in, assetnameb)))
                    .set((nft_table::confirmed.eq(true),))
                    .get_result::<Nft>(conn)?;

                return Ok(result);
            }
        }
        Err(RWDError::new("Mint of NFT cannot get confirmed"))
    }

    pub fn set_nft_payment_addr<'a>(
        conn: &PgConnection,
        id_in: &'a i64,
        assetnameb: &'a Vec<u8>,
        payment_addr_in: &'a String,
    ) -> Result<Nft, RWDError> {
        let result = diesel::update(nft_table::table.find((id_in, assetnameb)))
            .set(nft_table::payment_addr.eq(Some(payment_addr_in)))
            .get_result::<Nft>(conn)?;

        Ok(result)
    }

    pub fn set_nft_ipfs<'a>(
        conn: &PgConnection,
        id_in: &'a i64,
        assetnameb: &'a Vec<u8>,
        ipfs: &'a String,
    ) -> Result<Nft, RWDError> {
        let result = diesel::update(nft_table::table.find((id_in, assetnameb)))
            .set(nft_table::ipfs_hash.eq(Some(ipfs)))
            .get_result::<Nft>(conn)?;

        Ok(result)
    }

    pub fn set_nft_metadata<'a>(
        conn: &PgConnection,
        id_in: &'a i64,
        assetnameb: &'a Vec<u8>,
        metadata: &'a String,
    ) -> Result<Nft, RWDError> {
        let result = diesel::update(nft_table::table.find((id_in, assetnameb)))
            .set(nft_table::metadata.eq(metadata))
            .get_result::<Nft>(conn)?;

        Ok(result)
    }
}
