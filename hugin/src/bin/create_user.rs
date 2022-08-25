/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use hugin::drasildb::{TBContracts, TBDrasilUser};
use murin::chelper::*;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "fingerprint maker",
    about = "Creates Fingerprint from PolicyId and Tokenname"
)]
struct Opt {
    #[structopt(short, long, about = "password")]
    password: String,
    #[structopt(short, long, about = "user-id")]
    user_id: i64,
    #[structopt(short, long, about = "email")]
    email: String,
    #[structopt(
        short,
        long,
        about = "DrasilAdmin | Retailer | EnterpriseUser | StandardUser"
    )]
    role: String,
}

#[tokio::main]
async fn main() -> Result<(), MurinError> {
    let opt = Opt::from_args();
    let t = TBContracts::get_next_contract_id(&opt.user_id)?;
    println!("Established Connection Test: {:?}", t);

    let user = TBDrasilUser::create_user(
        None,
        &"dradmin".to_string(),
        &opt.email,
        &opt.password,
        &opt.role,
        &Vec::<String>::new(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &Vec::<String>::new(),
        None,
    )
    .await?;

    println!("Created user: {:?}", user);

    Ok(())
}
