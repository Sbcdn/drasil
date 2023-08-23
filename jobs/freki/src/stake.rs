use crate::models::*;
use crate::rwd_handling::handle_rewards;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use drasil_sleipnir::rewards::models::*;
use std::str::*;

pub(crate) async fn handle_stake(
    stake: drasil_mimir::EpochStakeView,
    twd: &TwlData,
    table: &mut Vec<RewardTable>,
) -> Result<()> {
    log::debug!("Handle Stake Address: {:?}", stake.stake_addr);
    if stake.amount.to_f64().unwrap() / 1000000.0 < 1.0 {
        return Ok(());
    }
    match twd.mode {
        drasil_gungnir::Calculationmode::RelationalToADAStake => {
            log::debug!("Calcualte with: RelationalToAdaStake");
            let mut token_earned = stake.amount * BigDecimal::from_str(&twd.equation)?;

            if let Some(modi) = &twd.modificator_equ {
                if let Ok(decimal) = serde_json::from_str::<BigDecimal>(modi) {
                    token_earned *= decimal
                };
            }

            handle_rewards(&stake.stake_addr, twd, &token_earned, table, false)?;
        }

        drasil_gungnir::Calculationmode::FixedEndEpoch => {
            log::debug!("Calcualte with: FixedEndEpoch");
            let x = if let Some(s) = twd.modificator_equ.clone() {
                BigDecimal::from_str(&s)?
            } else {
                BigDecimal::from_i32(0).unwrap()
            };
            log::debug!("X: {:?}", x);
            let y = BigDecimal::from_str(&twd.equation)?;
            log::debug!("Y: {:?}", y);
            let token_earned = y / x * stake.amount;
            handle_rewards(&stake.stake_addr, twd, &token_earned, table, false)?;
        }

        drasil_gungnir::Calculationmode::Custom => {
            match CustomCalculationTypes::from_str(&twd.equation).unwrap() {
                CustomCalculationTypes::Freeloaderz => {
                    log::debug!("Calculating Freeloaderz");
                    let adastake = stake.amount.to_f64().unwrap() / 1000000.0;
                    log::debug!(
                        "Ada Staked: {}, for addr: {} in epoch: {}",
                        adastake,
                        stake.stake_addr,
                        twd.calc_epoch
                    );
                    let param: FreeloaderzType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
                    if adastake > param.min_stake as f64 {
                        let token_earned = BigDecimal::from_f64(
                            ((adastake.powf(param.flatten)) + param.min_earned) * 1000000.0,
                        )
                        .unwrap()
                        .round(0);
                        log::debug!("Token earned before reward handle: {}", token_earned);
                        handle_rewards(&stake.stake_addr, twd, &token_earned, table, false)?;
                        log::debug!("Token earned after reward handle: {}", token_earned);
                    } else {
                        log::debug!("delegator below min stake");
                    }
                }
                CustomCalculationTypes::FixedAmountPerEpoch => {
                    log::debug!("Calcualte with: FixedAmountPerEpoch");
                    let adastake = stake.amount.to_f64().unwrap() / 1000000.0;
                    log::debug!(
                        "Ada Staked: {}, for addr: {} in epoch: {}",
                        adastake,
                        stake.stake_addr,
                        twd.calc_epoch
                    );
                    let param: FixedAmountPerEpochType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
                    if let Some(min) = param.min_stake {
                        if adastake > min {
                            handle_rewards(
                                &stake.stake_addr,
                                twd,
                                &BigDecimal::from_i128(param.amount * 1000000).unwrap(),
                                table,
                                false,
                            )?;
                        }
                    } else {
                        handle_rewards(
                            &stake.stake_addr,
                            twd,
                            &BigDecimal::from_i128(param.amount * 1000000).unwrap(),
                            table,
                            false,
                        )?;
                    }
                }
                CustomCalculationTypes::FixedAmountPerEpochNonAcc => {
                    log::debug!("Calcualte with: FixedAmountPerEpochNonAcc");

                    let adastake = stake.amount.to_f64().unwrap() / 1000000.0;
                    log::debug!(
                        "Ada Staked: {}, for addr: {} in epoch: {}",
                        adastake,
                        stake.stake_addr,
                        twd.calc_epoch
                    );
                    let param: FixedAmountPerEpochType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
                    if let Some(min) = param.min_stake {
                        if adastake > min {
                            handle_rewards(
                                &stake.stake_addr,
                                twd,
                                &BigDecimal::from_i128(param.amount * 1000000).unwrap(),
                                table,
                                true,
                            )?;
                        }
                    } else {
                        handle_rewards(
                            &stake.stake_addr,
                            twd,
                            &BigDecimal::from_i128(param.amount * 1000000).unwrap(),
                            table,
                            true,
                        )?;
                    }
                }
                CustomCalculationTypes::Threshold => {
                    log::debug!("Calcualte with: Threshold");

                    let adastake = stake.amount.to_f64().unwrap() / 1000000.0;
                    log::debug!(
                        "Ada Staked: {}, for addr: {} in epoch: {}",
                        adastake,
                        stake.stake_addr,
                        twd.calc_epoch
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
                        &BigDecimal::from_i128(token_earned).unwrap(),
                        table,
                        false,
                    )?;
                }
                CustomCalculationTypes::FixedAmountPerEpochCaped => {
                    log::debug!("Calcualte with: FixedAmountPerEpochCaped");
                    let adastake = stake.amount.to_i128().unwrap() / 1000000;
                    log::debug!(
                        "Ada Staked: {}, for addr: {} in epoch: {}",
                        adastake,
                        stake.stake_addr,
                        twd.calc_epoch
                    );

                    let param: CapedType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
                    let mul = adastake / param.cap_value;
                    let token_earned = mul * param.rwd * 1000000;
                    handle_rewards(
                        &stake.stake_addr,
                        twd,
                        &BigDecimal::from_i128(token_earned).unwrap(),
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
    pool: drasil_gungnir::GPools,
    epoch: i64,
    twd: &mut TwlData,
    table: &mut Vec<RewardTable>,
) -> Result<()> {
    log::debug!("Handle pool: {:?}", pool);
    let mut conn = drasil_mimir::establish_connection()?;
    let pool_stake = drasil_mimir::get_tot_stake_per_pool(&mut conn, &pool.pool_id, epoch as i32)?;
    for stake in pool_stake {
        handle_stake(stake, twd, table).await?;
    }

    Ok(())
}
