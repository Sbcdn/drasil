/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use gungnir::*;
use std::str::FromStr;
use chrono::{Utc, NaiveDateTime};
#[cfg(feature = "mimir_bin")]
use structopt::StructOpt;
#[cfg(feature = "mimir_bin")]
use mimir::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "rwd create token whitelisting", about = "Creates a Whitelisting for a token in a contract")]
struct Opt {
    //#[structopt(short, long, about="for stdout output set true")]
    //output: Option<bool>,

    #[structopt(short, long, about="user id as integer")]
    user: i32,

    #[structopt(short, long, about="contract id as integer")]
    contract_id: i32,

    #[structopt(short, long, about="policy id as String")]
    policy_id: String,

    #[structopt(short, long, about="tokenname as String")]
    tokenname: String,

    #[structopt(short, long, about="fingerprint as String")]
    fingerprint: Option<String>,

    #[structopt(short, long, about="Date until rewards are blocked")]
    vesting_period: Option<String>,

    #[structopt(short="l", long, about="pools array of Strings")]
    pools: Vec::<String>,

    #[structopt(short, long, about="[custom, modifactorandequation, simpleequation, fixedendepoch, relationaltoadastake]")]
    mode: String,

    #[structopt(short="q", long, about="Variable or equation, depends on mode")]
    equation: String,

    #[structopt(short, long, about="start epoch as integer")]
    start_epoch: i64,

    #[structopt(short, long, about="end epoch as integer")]
    end_epoch: Option<i64>,

    #[structopt(short="i", long, about="modificator depends on mode")]
    modificator: Option<String>,
}

/*
    pub fingerprint         : String,
    pub policy_id           : String,
    pub tokenname           : String,
    pub contract_id         : i64,
    pub user_id             : i64, 
    pub vesting_period      : DateTime<Utc>,
    pub pools               : Vec::<String>,
    pub mode                : Calculationmode,
    pub equation            : String,
    pub start_epoch         : i64,
    pub end_epoch           : Option<i64>,
    pub modificator_equ     : Option<String>,
*/
fn main() -> Result<(),RWDError> {
    //dotenv::dotenv().ok();
    //let database_url = env::var("REWARDS_DB_URL")?;
    let opt = Opt::from_args();

    let mconn = mimir::establish_connection()?;
    let fingerprint = mimir::get_fingerprint(&mconn, &opt.policy_id, &opt.tokenname)?;
    let tn =hex::encode(opt.tokenname.as_bytes());

    println!("PolicyId: {:?}", opt.policy_id);
    println!("TokenName: {:?}",tn);
    println!("Fingerprint: {:?}", fingerprint);

    println!("Pools: {:?}", opt.pools);

    let mut vd = chrono::Utc::now();
    if let Some(date) = opt.vesting_period {
        vd = chrono::DateTime::<Utc>::from_utc(NaiveDateTime::parse_from_str(&date, "%Y-%m-%d %H:%M:%S")?,Utc);
    }


    let gconn = gungnir::establish_connection()?;
    gungnir::TokenWhitelist::create_twl_entry(
        &gconn,
        &fingerprint,
        &opt.policy_id,
        &tn,
        &(opt.contract_id as i64),
        &(opt.user as i64),
        &vd,
        &opt.pools,
        &gungnir::Calculationmode::from_str(&opt.mode)?,
        &opt.equation,
        &opt.start_epoch,
        opt.end_epoch.as_ref(),
        opt.modificator.as_ref(),
    )?;


    Ok(())
}