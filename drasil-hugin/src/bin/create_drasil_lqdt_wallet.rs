extern crate tokio;
use drasil_hugin::database::drasildb::{error::SystemDBError, TBContracts, TBMultiSigLoc};
use drasil_murin::*;
use structopt::StructOpt;
use zeroize::Zeroize;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "rwd_multi_wallet_creator",
    about = "Creates Multi-Sig Wallet for the use with drasil."
)]
struct Opt {
    #[structopt(short, long, about = "for stdout output set true")]
    output: Option<bool>,

    #[structopt(short, long, about = "if testnet contract set true")]
    testnet: Option<bool>,

    #[structopt(short, long, about = "user id as integer")]
    user: i64,

    #[structopt(short, long, about = "contract id as integer")]
    contract_id: i64,
}
pub fn harden(num: u32) -> u32 {
    0x80000000 + num
}

#[tokio::main]
async fn main() -> Result<(), SystemDBError> {
    let opt = Opt::from_args();

    let mut net_bytes = 0b0001;
    if opt.testnet.is_some() {
        println!("Got testnet");
        net_bytes = 0b0000;
    }

    let wallet = drasil_murin::wallet::create_wallet();

    let stakecred = clib::address::StakeCredential::from_keyhash(&wallet.3);
    let e_address = clib::address::EnterpriseAddress::new(net_bytes, &stakecred);
    let address = e_address.to_address();
    let bech32_address = address.to_bech32(None)?;

    let mut str_wallet = String::new();
    str_wallet.push_str(&hex::encode(wallet.0.as_bytes()));
    str_wallet.push('|');
    str_wallet.push_str(&hex::encode(wallet.1.as_bytes()));
    str_wallet.push('|');
    str_wallet.push_str(&hex::encode(wallet.2.as_bytes()));
    str_wallet.push('|');
    str_wallet.push_str(&hex::encode(wallet.3.to_bytes()));
    str_wallet.push('|');
    str_wallet.push_str(&wallet.4);
    str_wallet.push('|');
    str_wallet.push_str(&wallet.5);

    let mut password = rpassword::prompt_password_stdout("password:").unwrap();
    let wallet_encr = drasil_hugin::encryption::encrypt(&str_wallet, &password).unwrap();

    let contract_type = "drasilliquidity".to_string();
    let description = Some("Drasil Liquidity Wallet");
    let _ = TBContracts::create_contract(
        &opt.user,
        &opt.contract_id,
        &contract_type,
        description,
        &(1.0),
        &wallet_encr,
        &bech32_address,
        None,
        &false,
    )?;
    password.zeroize();
    let pvks = vec![hex::encode(wallet.0.as_bytes())];

    let _ = TBMultiSigLoc::create_multisig_keyloc(
        &opt.user,
        &opt.contract_id,
        &(1.0),
        &bech32_address,
        None,
        None,
        &pvks,
        &false,
    )
    .await?;

    if opt.output.is_some() {
        println!("Encrypted Wallet Data: {wallet_encr}");
        println!("Wallet Address: {bech32_address:?}");
        println!("Public Key: {:?}", hex::encode(wallet.3.to_bytes()));
        println!("Vkey: {:?}", wallet.4);
        println!("Skey: {:?}", wallet.5);
    }

    println!("Decrypt: \n");
    let cipher = rpassword::prompt_password_stdout("cipher:")?;
    let mut password = rpassword::prompt_password_stdout("password:")?;
    let wallet_decr =
        drasil_hugin::encryption::decrypt(&cipher, &password).expect("Could not encrypt data");
    password.zeroize();

    println!("Decrypted: \n{wallet_decr}");

    Ok(())
}