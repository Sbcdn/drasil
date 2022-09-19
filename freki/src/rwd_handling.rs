/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use super::models::*;
use crate::stake::handle_pool;
use crate::whitelists::handle_whitelist;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use std::str::*;

pub async fn get_token_whitelist(current_epoch: i64) -> Result<Vec<gungnir::TokenWhitelist>> {
    let whitelist = gungnir::TokenWhitelist::get_epoch_filtered_whitelist(current_epoch)?;

    Ok(whitelist)
}

pub(crate) fn check_contract_is_active(twle: &gungnir::TokenWhitelist) -> Result<bool> {
    let contr = hugin::database::TBContracts::get_contract_uid_cid(twle.user_id, twle.contract_id)?;

    if !contr.depricated {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(crate) fn handle_rewards(
    stake_addr: &String,
    twd: &TwlData,
    token_earned: &BigDecimal,
    table: &mut Vec<RewardTable>,
    no_acc: bool,
) -> Result<()> {
    let mut gconn = gungnir::establish_connection()?;
    println!("Try to find rewards...");
    let rewards = gungnir::Rewards::get_rewards_per_token(
        &mut gconn,
        stake_addr,
        twd.contract_id,
        twd.user_id,
        &twd.fingerprint.clone(),
    )?;
    let mut tot_earned = BigDecimal::from_i32(0).unwrap();
    if rewards.len() == 1 && rewards[0].last_calc_epoch < twd.calc_epoch {
        if no_acc
            && gungnir::Rewards::get_available_rewards(
                &mut gconn,
                &rewards[0].stake_addr,
                &rewards[0].payment_addr,
                &rewards[0].fingerprint,
                rewards[0].contract_id,
                rewards[0].user_id,
                token_earned.to_i64().unwrap(),
            )? != -token_earned.to_i64().unwrap()
        {
            gungnir::Rewards::update_rewards(
                &mut gconn,
                &rewards[0].stake_addr,
                &rewards[0].fingerprint,
                &rewards[0].contract_id,
                &rewards[0].user_id,
                &rewards[0].tot_earned,
                &twd.calc_epoch,
            )?;
            return Ok(());
        }
        tot_earned = rewards[0].tot_earned.clone() + token_earned.clone();
        println!("Earned add: {:?}", tot_earned);
        let stake_rwd = gungnir::Rewards::update_rewards(
            &mut gconn,
            &rewards[0].stake_addr,
            &rewards[0].fingerprint,
            &rewards[0].contract_id,
            &rewards[0].user_id,
            &tot_earned,
            &twd.calc_epoch,
        )?;
        println!("Stake Rewards Added : {:?}", stake_rwd);
    }
    if rewards.is_empty() {
        let payment_addr = mimir::api::select_addr_of_first_transaction(stake_addr)?;

        tot_earned = token_earned.to_owned();
        println!("Earned new: {:?}", tot_earned);
        let stake_rwd = gungnir::Rewards::create_rewards(
            &mut gconn,
            stake_addr,
            &payment_addr,
            &twd.fingerprint,
            &twd.contract_id,
            &twd.user_id,
            &tot_earned,
            &BigDecimal::from_i32(0).unwrap(),
            &false,
            &twd.calc_epoch,
        );
        println!("Stake Rewards New: {:?}", stake_rwd);
    }
    if rewards.len() > 1 {
        return Err(murin::MurinError::new(
            "More than one reward entry found on the same contract for the same token",
        )
        .into());
    }
    // Store reward calculation to csv
    let table_entry = RewardTable {
        twldata: twd.clone(),
        calc_date: chrono::offset::Utc::now(),
        calc_epoch: twd.calc_epoch,
        current_epoch: twd.calc_epoch + 2,
        earned_epoch: token_earned.clone(),
        total_earned_epoch: tot_earned,
    };
    table.push(table_entry);
    Ok(())
}

pub(crate) async fn handle_lists(
    rwd_token: &mut gungnir::TokenWhitelist,
    epoch: i64,
    table: &mut Vec<RewardTable>,
) -> Result<()> {
    let spools = rwd_token.pools.clone();
    let mut pools = Vec::<gungnir::GPools>::new();
    pools.extend(
        spools
            .iter()
            .map(|n| gungnir::GPools::from_str(n).expect("Could not convert string to GPools")),
    );
    pools.retain(|p| p.first_valid_epoch <= epoch);

    let mut whitelists = pools.clone();
    whitelists.retain(|w| WhitelistLink::is_wl_link(&w.pool_id));
    let mut wlists = Vec::<WhitelistLink>::new();
    whitelists
        .iter()
        .for_each(|n| wlists.push(WhitelistLink::from_str(&n.pool_id).unwrap()));

    let mut conn = mimir::establish_connection()?;

    // Get total Ada staked from all participating pools
    match rwd_token.mode.clone() {
        gungnir::Calculationmode::FixedEndEpoch => {
            let mut total_pools_stake = 0;
            for pool in pools.clone() {
                total_pools_stake +=
                    mimir::get_pool_total_stake(&mut conn, &pool.pool_id, epoch as i32)? / 1000000
            }
            rwd_token.modificator_equ = Some(total_pools_stake.to_string());
        }
        gungnir::Calculationmode::AirDrop => {
            return Ok(());
        }

        _ => {}
    }

    // Hanlde Whitelists
    for whitelist in wlists {
        let mut twd = TwlData::new(
            rwd_token.fingerprint.clone().unwrap(),
            rwd_token.policy_id.clone(),
            rwd_token.tokenname.clone().unwrap(),
            rwd_token.contract_id,
            rwd_token.user_id,
            rwd_token.vesting_period,
            AddrSrc::Whitelist(whitelist.clone()),
            rwd_token.mode.clone(),
            rwd_token.equation.clone(),
            rwd_token.start_epoch,
            rwd_token.end_epoch,
            rwd_token.modificator_equ.clone(),
            epoch,
        );
        handle_whitelist(whitelist, &mut twd, table).await?;
    }

    //Hanlde Stakepools
    for pool in pools {
        let mut twd = TwlData::new(
            rwd_token.fingerprint.clone().unwrap(),
            rwd_token.policy_id.clone(),
            rwd_token.tokenname.clone().unwrap(),
            rwd_token.contract_id,
            rwd_token.user_id,
            rwd_token.vesting_period,
            AddrSrc::GPools(pool.clone()),
            rwd_token.mode.clone(),
            rwd_token.equation.clone(),
            rwd_token.start_epoch,
            rwd_token.end_epoch,
            rwd_token.modificator_equ.clone(),
            epoch,
        );
        handle_pool(pool, epoch, &mut twd, table).await?; //npools
    }

    Ok(())
}
