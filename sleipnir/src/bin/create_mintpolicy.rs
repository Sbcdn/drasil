/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use chrono::{NaiveDateTime, Utc};
use sleipnir::*;
use std::str::FromStr;
#[cfg(feature = "clib")]
use structopt::StructOpt;

/*
[features]
clib = ["structopt"]

[[bin]]
name = "create_mintpolicy"
required-features = ["clib"]
 */

#[derive(Debug, StructOpt)]
#[structopt(
    name = "create minting policy",
    about = "Creates a minting policy contract"
)]
struct Opt {
    //#[structopt(short, long, about="for stdout output set true")]
    //output: Option<bool>,
    #[structopt(short, long, about = "User id as integer")]
    user: i32,

    #[structopt(short, long, about = "Optional: Testnet -> true")]
    network: Option<bool>,

    #[structopt(
        short,
        long,
        about = "Optional: Wallet Bech32-Address which needs to sign the mint"
    )]
    signer: Option<String>,

    #[structopt(
        short,
        long,
        about = "Optionla: Date until for timelocking the policy-id"
    )]
    timelock: Option<String>,
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
fn main() -> Result<(), SleipnirError> {
    //dotenv::dotenv().ok();
    //let database_url = env::var("DRASIL_REWARD_DB")?;
    let opt = Opt::from_args();

    let mut vd = None;
    if let Some(date) = opt.timelock {
        vd = Some(chrono::DateTime::<Utc>::from_utc(
            NaiveDateTime::parse_from_str(&date, "%Y-%m-%d %H:%M:%S")?,
            Utc,
        ));
    }

    let mut signer = None;
    if let Some(a) = opt.signer {
        signer = Some(murin::clib::address::Address::from_bech32(&a)?);
    }

    let mut network = murin::clib::NetworkIdKind::Mainnet;
    if let Some(_) = opt.network {
        network = murin::clib::NetworkIdKind::Testnet;
    }

    sleipnir::minting::create_policy_script(network, opt.user, signer, vd)?;

    Ok(())
}
