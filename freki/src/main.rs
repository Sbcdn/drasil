/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
mod models;

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, Utc};
use core::fmt;
use csv::WriterBuilder;
use std::str::*;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Reward Calculator",
    about = "Calculates rewards for the drasil - freeloaderz SmartClaimz system."
)]
struct Opt {
    #[structopt(short, long, about = "the epoch rewards should be calcualted for")]
    epoch: Option<i64>,

    #[structopt(
        short,
        long,
        about = "calc from the given epoch up to the latest possible one"
    )]
    from: Option<bool>,

    #[structopt(short, long, about = "the epoch rewards should be calcualted for")]
    t: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct TwlData {
    pub fingerprint: String,
    pub policy_id: String,
    pub tokenname: String,
    pub contract_id: i64,
    pub user_id: i64,
    pub vesting_period: DateTime<Utc>,
    pub pool: gungnir::GPools,
    pub mode: gungnir::Calculationmode,
    pub equation: String,
    pub start_epoch: i64,
    pub end_epoch: Option<i64>,
    pub modificator_equ: Option<String>,
    pub calc_epoch: i64,
}

impl TwlData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        fingerprint: String,
        policy_id: String,
        tokenname: String,
        contract_id: i64,
        user_id: i64,
        vesting_period: DateTime<Utc>,
        pool: gungnir::GPools,
        mode: gungnir::Calculationmode,
        equation: String,
        start_epoch: i64,
        end_epoch: Option<i64>,
        modificator_equ: Option<String>,
        calc_epoch: i64,
    ) -> TwlData {
        TwlData {
            fingerprint,
            policy_id,
            tokenname,
            contract_id,
            user_id,
            vesting_period,
            pool,
            mode,
            equation,
            start_epoch,
            end_epoch,
            modificator_equ,
            calc_epoch,
        }
    }

    pub fn to_str_vec(&self) -> Vec<String> {
        vec![
            self.fingerprint.clone(),
            self.policy_id.clone(),
            self.tokenname.clone(),
            self.contract_id.to_string(),
            self.user_id.to_string(),
            self.vesting_period.to_string(),
            self.pool.to_string(),
            self.mode.to_string(),
            self.equation.clone(),
            self.start_epoch.to_string(),
            self.end_epoch.unwrap_or(0).to_string(),
            self.modificator_equ
                .clone()
                .unwrap_or_else(|| (&"None").to_string()),
            self.calc_epoch.to_string(),
        ]
    }
}
impl fmt::Display for TwlData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},", self.fingerprint)?;
        write!(f, "{},", self.policy_id)?;
        write!(f, "{},", self.tokenname)?;
        write!(f, "{},", self.contract_id)?;
        write!(f, "{},", self.user_id)?;
        write!(f, "{},", self.vesting_period)?;
        write!(f, "{},", self.pool)?;
        write!(f, "{},", self.mode.to_string())?;
        write!(f, "{},", self.equation)?;
        write!(f, "{},", self.start_epoch)?;
        write!(f, "{},", self.end_epoch.unwrap_or(0))?;
        write!(
            f,
            "{},",
            self.modificator_equ.as_ref().unwrap_or(&"None".to_string())
        )?;
        write!(f, "{}", self.calc_epoch)
    }
}
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct RewardTable {
    pub twldata: TwlData,
    pub calc_date: DateTime<Utc>,
    pub calc_epoch: i64,
    pub current_epoch: i64,
    pub earned_epoch: BigDecimal,
    pub total_earned_epoch: BigDecimal,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn handle_rewards(
    stake_addr: &String,
    twd: &TwlData,
    token_earned: &BigDecimal,
    table: &mut Vec<RewardTable>,
) -> Result<()> {
    let gconn = gungnir::establish_connection()?;
    let rewards = gungnir::Rewards::get_rewards_per_token(
        &gconn,
        stake_addr,
        twd.contract_id,
        twd.user_id,
        &twd.fingerprint.clone(),
    )?;
    let mut tot_earned = BigDecimal::from_i32(0).unwrap();
    if rewards.len() == 1
        && rewards[0].fingerprint == twd.fingerprint
        && rewards[0].last_calc_epoch < twd.calc_epoch
    {
        tot_earned = rewards[0].tot_earned.clone() + token_earned.clone();
        println!("Earned: {:?}", tot_earned);
        let stake_rwd = gungnir::Rewards::update_rewards(
            &gconn,
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
        println!("Earned: {:?}", tot_earned);
        let stake_rwd = gungnir::Rewards::create_rewards(
            &gconn,
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

pub(crate) async fn handle_stake(
    stake: mimir::EpochStakeView,
    twd: &TwlData,
    table: &mut Vec<RewardTable>,
) -> Result<()> {
    println!("Handle Stake Address: {:?}", stake.stake_addr);
    //let gconn = gungnir::establish_connection()?;
    //let lovelace = BigDecimal::from_i32(1000000).unwrap();
    match twd.mode {
        gungnir::Calculationmode::RelationalToADAStake => {
            println!("Calcualte with: RelationalToAdaStake");
            let token_earned = stake.amount * BigDecimal::from_str(&twd.equation)?;
            handle_rewards(&stake.stake_addr, twd, &token_earned, table)?;
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
            handle_rewards(&stake.stake_addr, twd, &token_earned, table)?;
        }

        gungnir::Calculationmode::SimpleEquation => {}

        gungnir::Calculationmode::ModifactorAndEquation => {}

        gungnir::Calculationmode::Custom => {
            use crate::models::*;
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
                        .unwrap();
                        handle_rewards(&stake.stake_addr, twd, &token_earned, table)?;
                        println!("Total earned: {}", token_earned);
                    } else {
                        println!("delegator below min stake");
                    }
                }
            }
        }
        gungnir::Calculationmode::AirDrop => {
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
    let conn = mimir::establish_connection()?;
    let pool_stake = mimir::get_tot_stake_per_pool(&conn, &pool.pool_id, epoch as i32)?;
    for stake in pool_stake {
        handle_stake(stake, twd, table).await?;
    }

    Ok(())
}

pub(crate) async fn handle_pools(
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

    let conn = mimir::establish_connection()?;

    // Get total Ada staked from all participating pools
    match rwd_token.mode.clone() {
        gungnir::Calculationmode::FixedEndEpoch => {
            let mut total_pools_stake = 0;
            for pool in pools.clone() {
                total_pools_stake +=
                    mimir::get_pool_total_stake(&conn, &pool.pool_id, epoch as i32)? / 1000000
            }
            rwd_token.modificator_equ = Some(total_pools_stake.to_string());
        }
        gungnir::Calculationmode::AirDrop => {
            return Ok(());
        }

        _ => {}
    }

    for pool in pools {
        let mut twd = TwlData::new(
            rwd_token.fingerprint.clone().unwrap(),
            rwd_token.policy_id.clone(),
            rwd_token.tokenname.clone().unwrap(),
            rwd_token.contract_id,
            rwd_token.user_id,
            rwd_token.vesting_period,
            pool.clone(),
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

pub async fn get_token_whitelist(current_epoch: i64) -> Result<Vec<gungnir::TokenWhitelist>> {
    let whitelist = gungnir::TokenWhitelist::get_epoch_filtered_whitelist(current_epoch)?;

    Ok(whitelist)
}

fn check_contract_is_active(twle: &gungnir::TokenWhitelist) -> Result<bool> {
    let contr = hugin::database::TBContracts::get_contract_uid_cid(twle.user_id, twle.contract_id)?;

    if !contr.depricated {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tokio::main]
pub async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let opt = Opt::from_args();

    if opt.t.is_some() {
        let flz = models::CustomCalculationTypes::Freeloaderz;
        let data = models::FreeloaderzType {
            min_stake: 150,
            min_earned: 50.0,
            flatten: 0.6,
        };
        println!("ENUM: {}", flz);
        println!("FLZType: \n{}", serde_json::json!(data));
        return Ok(());
    }

    let current_epoch = mimir::get_epoch(&mimir::establish_connection()?)? as i64;
    let calc_epoch = current_epoch - 2;
    println!("Current Epoch: {}", current_epoch);
    println!("Calculation Epoch: {}", calc_epoch);
    if opt.epoch.is_some() && opt.epoch.unwrap() > calc_epoch {
        return Err(gungnir::RWDError::new(
            "It is not possible to calculate rewards for the current or future epochs",
        )
        .into());
    }

    let mut i = calc_epoch;
    if opt.epoch.is_some() {
        i = opt.epoch.unwrap();
    };
    let mut table = Vec::<RewardTable>::new();
    if let Some(b) = opt.from {
        while i < calc_epoch && b {
            let mut whitelist = get_token_whitelist(calc_epoch).await?;
            whitelist.retain(|w| w.start_epoch <= i);
            println!("Whitelist: {:?}", whitelist);
            for mut entry in whitelist {
                if check_contract_is_active(&entry)? {
                    println!("Entered: {:?}", entry);
                    handle_pools(&mut entry, i, &mut table).await?
                    //   tokio::spawn(async move {
                    //       if let Err(err) = handle_pools(&mut entry,i).await {
                    //error!(cause = ?err, "calculation error for whitelist entry");
                    //           panic!("calculation error for whitelist entry: {:?}",err);
                    //       }
                    //   });
                }
            }
            i += 1;
        }
        println!(
            "Rewards successfully calucalted for epochs {:?} to {:?}",
            opt.epoch, i
        );
    } else {
        let mut whitelist = get_token_whitelist(calc_epoch).await?;
        whitelist.retain(|w| w.start_epoch <= i);
        println!("Whitelist: {:?}", whitelist);
        for mut entry in whitelist {
            if check_contract_is_active(&entry)? {
                handle_pools(&mut entry, i, &mut table).await?
                //tokio::spawn(async move {
                //    if let Err(err) = handle_pools(&mut entry,opt.epoch).await {
                //        //error!(cause = ?err, "calculation error for whitelist entry");
                //        panic!("calculation error for whitelist entry: {:?}",err);
                //    }
                //});
            }
        }
        println!("Rewards successfully calucalted for epoch: {:?}", i);
    }
    let mut path =
        std::env::var("CSV_PATH").expect("Could not open CSV path, environment variable not set");
    path.push_str(&(calc_epoch.to_string() + "_"));
    path.push_str(&chrono::offset::Utc::now().to_string());
    let mut wtr = WriterBuilder::new().from_path(path + "{}")?;
    let mut wtr2 = WriterBuilder::new().from_writer(vec![]);
    for entry in table {
        let mut e = entry.twldata.to_str_vec();
        e.extend(
            &mut vec![
                entry.calc_date.to_string(),
                entry.current_epoch.to_string(),
                entry.earned_epoch.to_string(),
                entry.total_earned_epoch.to_string(),
            ]
            .into_iter(),
        );
        wtr.write_record(&e)?;
        wtr2.write_record(&e)?;
    }
    wtr.flush()?;

    let data = String::from_utf8(wtr2.into_inner()?)?;
    println!("{:?}", data);
    Ok(())
}
