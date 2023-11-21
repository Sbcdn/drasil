use argon2::{Argon2, password_hash::SaltString, PasswordHasher};
use drasil_hugin::{
    drasildb::{TBContracts, TBDrasilUser},
    error::SystemDBError,
};
use rand::rngs::OsRng;
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
}

#[tokio::main]
async fn main() -> Result<(), SystemDBError> {
    let opt = Opt::from_args();
    //let t = TBContracts::get_next_contract_id(&opt.user_id)?;
    //println!("Established Connection Test: {t:?}");

    let password_hash = Argon2::default()
            .hash_password(&opt.password.as_bytes(), &SaltString::generate(&mut OsRng))?
            .to_string();

    /*
    let user = TBDrasilUser::create_user(
        None,
        &"dradmin".to_string(),
        &opt.email,
        &password_hash,
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
     */
    //println!("Created user: {user:?}");
    println!("Created user: {password_hash:?}");

    Ok(())
}
