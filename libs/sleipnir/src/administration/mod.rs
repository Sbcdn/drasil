pub use crate::error::SleipnirError;
use hugin::database::{TBContracts, TBMultiSigLoc};
use hugin::encryption::{generate_pph, mident};
use murin::clib::address::{EnterpriseAddress, StakeCredential};
use zeroize::Zeroize;

pub async fn create_lqdt_wallet(
    network: murin::clib::NetworkIdKind,
    user: &i64,
) -> Result<String, SleipnirError> {
    let mut net_bytes = 0b0001;
    if network == murin::clib::NetworkIdKind::Testnet {
        net_bytes = 0b0000;
    }
    let wallet = murin::wallet::create_wallet();

    let stakecred = StakeCredential::from_keyhash(&wallet.3);
    let e_address = EnterpriseAddress::new(net_bytes, &stakecred);
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

    let ci = TBContracts::get_next_contract_id(user)?;

    let ident = mident(user, &ci, &0.0, &bech32_address);
    let mut password = generate_pph(&ident).await;
    let wallet_encr = hugin::encryption::encrypt(&str_wallet, &password).unwrap();
    password.zeroize();

    if TBContracts::get_liquidity_wallet(user).is_ok() {
        return Err(SleipnirError::new(
            "This user already has a liquidity wallet",
        ));
    };

    let _ = TBContracts::create_contract(
        user,
        &ci,
        "drasilliquidity",
        Some(&("Liquidity Wallet: ".to_owned() + &user.to_string())),
        &(1.0),
        &wallet_encr,
        &bech32_address,
        None,
        &false,
    )?;

    let pvks = vec![hex::encode(wallet.0.as_bytes())];

    let _ = TBMultiSigLoc::create_multisig_keyloc(
        user,
        &ci,
        &(1.0),
        &bech32_address,
        None,
        None,
        &pvks,
        &false,
    )
    .await?;

    Ok(bech32_address)
}
