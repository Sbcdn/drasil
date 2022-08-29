/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use murin::chelper::*;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "fingerprint maker",
    about = "Creates Fingerprint from PolicyId and Tokenname"
)]
struct Opt {
    #[structopt(short, long, about = "policy ID in hey")]
    policy: String,

    #[structopt(short, long, about = "if testnet contract set true")]
    tokenname: Option<String>,
}

fn main() -> Result<(), MurinError> {
    let opt = Opt::from_args();
    let tn = match opt.tokenname {
        Some(t) => t,
        None => "".to_string(),
    };
    let fp = make_fingerprint(&opt.policy, &tn)?;

    println!("Fingerprint: {:?}", fp);

    Ok(())
}
