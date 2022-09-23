/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use crate::models::*;
use crate::rwd_handling::handle_rewards;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use sleipnir::rewards::models::*;
use std::str::*;

pub(crate) async fn handle_stake(
    stake: mimir::EpochStakeView,
    twd: &TwlData,
    table: &mut Vec<RewardTable>,
) -> Result<()> {
    println!("Handle Stake Address: {:?}", stake.stake_addr);
    match twd.mode {
        gungnir::Calculationmode::RelationalToADAStake => {
            println!("Calcualte with: RelationalToAdaStake");
            let token_earned = stake.amount * BigDecimal::from_str(&twd.equation)?;
            handle_rewards(&stake.stake_addr, twd, &token_earned, table, false)?;
        }

        gungnir::Calculationmode::FixedEndEpoch => {
            println!("Calcualte with: FixedEndEpoch");
            let x = if let Some(s) = twd.modificator_equ.clone() {
                BigDecimal::from_str(&s)?
            } else {
                BigDecimal::from_i32(1).unwrap()
            }; //total at stake
            println!("X: {:?}", x);
            let y = BigDecimal::from_str(&twd.equation)?;
            println!("Y: {:?}", y);
            let token_earned = y / x * stake.amount;
            handle_rewards(&stake.stake_addr, twd, &token_earned, table, false)?;
        }

        gungnir::Calculationmode::Custom => {
            //Freeloaderz
            match CustomCalculationTypes::from_str(&twd.equation).unwrap() {
                //R=(S-150)^0.6+50 where R=payout in FLZ per epoch and S=Stake Amount to the pool. Example
                CustomCalculationTypes::Freeloaderz => {
                    println!("Calculating Freeloaderz");
                    let adastake = stake.amount.to_f64().unwrap() / 1000000.0;
                    println!(
                        "Ada Staked: {}, for addr: {} in epoch: {}",
                        adastake, stake.stake_addr, twd.calc_epoch
                    );
                    let param: FreeloaderzType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
                    if adastake > param.min_stake as f64 {
                        let token_earned = BigDecimal::from_f64(
                            ((adastake.powf(param.flatten)) + param.min_earned) * 1000000.0,
                        )
                        .unwrap()
                        .round(0);
                        println!("Token earned before reward handle: {}", token_earned);
                        handle_rewards(&stake.stake_addr, twd, &token_earned, table, false)?;
                        println!("Token earned after reward handle: {}", token_earned);
                    } else {
                        println!("delegator below min stake");
                    }
                }
                CustomCalculationTypes::FixedAmountPerEpoch => {
                    println!("Calcualte with: FixedAmountPerEpoch");
                    let adastake = stake.amount.to_f64().unwrap() / 1000000.0;
                    println!(
                        "Ada Staked: {}, for addr: {} in epoch: {}",
                        adastake, stake.stake_addr, twd.calc_epoch
                    );
                    let param: FixedAmountPerEpochType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
                    if let Some(min) = param.min_stake {
                        if adastake > min {
                            handle_rewards(
                                &stake.stake_addr,
                                twd,
                                &BigDecimal::from_u64(param.amount * 1000000).unwrap(),
                                table,
                                false,
                            )?;
                        }
                    } else {
                        handle_rewards(
                            &stake.stake_addr,
                            twd,
                            &BigDecimal::from_u64(param.amount * 1000000).unwrap(),
                            table,
                            false,
                        )?;
                    }
                }
                CustomCalculationTypes::FixedAmountPerEpochNonAcc => {
                    println!("Calcualte with: FixedAmountPerEpochNonAcc");
                    let adastake = stake.amount.to_f64().unwrap() / 1000000.0;
                    println!(
                        "Ada Staked: {}, for addr: {} in epoch: {}",
                        adastake, stake.stake_addr, twd.calc_epoch
                    );
                    let param: FixedAmountPerEpochType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
                    if let Some(min) = param.min_stake {
                        if adastake > min {
                            handle_rewards(
                                &stake.stake_addr,
                                twd,
                                &BigDecimal::from_u64(param.amount * 1000000).unwrap(),
                                table,
                                true,
                            )?;
                        }
                    } else {
                        handle_rewards(
                            &stake.stake_addr,
                            twd,
                            &BigDecimal::from_u64(param.amount * 1000000).unwrap(),
                            table,
                            true,
                        )?;
                    }
                }
                CustomCalculationTypes::Threshold => {
                    println!("Calcualte with: Threshold");
                    let adastake = stake.amount.to_f64().unwrap() / 1000000.0;
                    println!(
                        "Ada Staked: {}, for addr: {} in epoch: {}",
                        adastake, stake.stake_addr, twd.calc_epoch
                    );
                    let param: ThresholdType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
                    let token_earned = if adastake >= param.stake_threshold {
                        param.upper_rwd * 1000000
                    } else {
                        param.lower_rwd * 1000000
                    };
                    handle_rewards(
                        &stake.stake_addr,
                        twd,
                        &BigDecimal::from_u64(token_earned).unwrap(),
                        table,
                        false,
                    )?;
                }
                CustomCalculationTypes::Airdrop => {}
            }
        }
        _ => {
            //Nothing to Do
        }
    }

    Ok(())
}

pub(crate) async fn handle_pool(
    pool: gungnir::GPools,
    epoch: i64,
    twd: &mut TwlData,
    table: &mut Vec<RewardTable>,
) -> Result<()> {
    println!("Handle pool: {:?}", pool);
    let mut conn = mimir::establish_connection()?;
    let pool_stake = mimir::get_tot_stake_per_pool(&mut conn, &pool.pool_id, epoch as i32)?;
    for stake in pool_stake {
        handle_stake(stake, twd, table).await?;
    }

    Ok(())
}
