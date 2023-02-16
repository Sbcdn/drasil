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
use bigdecimal::{BigDecimal, FromPrimitive};
use sleipnir::rewards::models::*;
use std::str::*;

pub(crate) async fn handle_whitelist_address(
    addr: &String,
    twd: &TwlData,
    table: &mut Vec<RewardTable>,
) -> Result<()> {
    let stake_addr = murin::get_reward_address(&murin::b_decode_addr(addr).await?)?
        .to_bech32(None)
        .unwrap_or_else(|_| addr.clone());
    let script_reward = *addr == stake_addr;
    if script_reward {
        return Err(Box::new(murin::MurinError::new(
            "Scritp Rewards not implemented yet",
        )));
    }

    match twd.mode {
        gungnir::Calculationmode::AirDrop => {
            todo!();
            //This is a reoccuring airdrop, add new rewards
            //let param: ReoccuringAirdrop =
            //            serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
        }
        gungnir::Calculationmode::Custom => {
            match CustomCalculationTypes::from_str(&twd.equation).unwrap() {
                CustomCalculationTypes::FixedAmountPerEpoch => {
                    log::debug!("Whitelist calcualte with: FixedAmountPerEpoch");
                    let param: FixedAmountPerEpochType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;

                    handle_rewards(
                        &stake_addr,
                        twd,
                        &BigDecimal::from_i128(param.amount * 1000000).unwrap(),
                        table,
                        false,
                    )?;
                }
                CustomCalculationTypes::FixedAmountPerEpochNonAcc => {
                    log::debug!("Whitelist calcualte with: FixedAmountPerEpochNonAcc");
                    let param: FixedAmountPerEpochType =
                        serde_json::from_str(&twd.modificator_equ.clone().unwrap())?;
                    handle_rewards(
                        &stake_addr,
                        twd,
                        &BigDecimal::from_i128(param.amount * 1000000).unwrap(),
                        table,
                        true,
                    )?;
                }
                _ => {
                    // stake related calculation modes are not supported for whitelists
                }
            }
        }
        _ => {
            // stake related calculation modes are not supported for whitelists
        }
    }
    Ok(())
}

pub(crate) async fn handle_whitelist(
    wl_link: WhitelistLink,
    twd: &mut TwlData,
    table: &mut Vec<RewardTable>,
) -> Result<()> {
    log::debug!("Handle whitelist: {:?}", wl_link);
    let addr_list = gungnir::WlAlloc::get_whitelist(&wl_link.id)?;
    for addr in addr_list {
        handle_whitelist_address(&addr, twd, table).await?;
    }

    Ok(())
}
