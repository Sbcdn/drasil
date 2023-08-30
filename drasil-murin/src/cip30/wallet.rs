use crate::error::MurinError;
use crate::htypes::*;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto};
use clib::address::{BaseAddress, EnterpriseAddress};
use clib::crypto::{Ed25519KeyHash, PrivateKey, PublicKey};

/// decode an hex encoded address into an address
pub async fn decode_addr(bytes: &String) -> Result<caddr::Address, MurinError> {
    //let stake_cred_key = ccrypto::Ed25519KeyHash::from_bytes(hex::decode(bytes)?)?;
    //let stake_cred = caddr::StakeCredential::from_keyhash(&stake_cred_key);
    //let mut netbyte : u8 = 0b1111;
    //if *network == clib::NetworkIdKind::Testnet {
    //     netbyte = 0b1110;
    //}
    Ok(caddr::Address::from_bytes(hex::decode(bytes)?)?)
    //Ok(caddr::RewardAddress::new(netbyte,&stake_cred).to_address())
}

/// decode an hex encoded address into an address
pub async fn b_decode_addr(str: &String) -> Result<caddr::Address, MurinError> {
    //let stake_cred_key = ccrypto::Ed25519KeyHash::from_bytes(hex::decode(bytes)?)?;
    //let stake_cred = caddr::StakeCredential::from_keyhash(&stake_cred_key);
    //let mut netbyte : u8 = 0b1111;
    //if *network == clib::NetworkIdKind::Testnet {
    //     netbyte = 0b1110;
    //}
    match hex::decode(str) {
        Ok(bytes) => Ok(caddr::Address::from_bytes(bytes)?),
        Err(_) => match caddr::Address::from_bech32(str) {
            Ok(addr) => Ok(addr),
            Err(_e) => Err(MurinError::new(
                "The provided Address is not byte encoded not bech32 encoded, Address invalid!",
            )),
        },
    }
    //Ok(caddr::RewardAddress::new(netbyte,&stake_cred).to_address())
}

pub fn address_from_string_non_async(str: &String) -> Result<caddr::Address, MurinError> {
    //let stake_cred_key = ccrypto::Ed25519KeyHash::from_bytes(hex::decode(bytes)?)?;
    //let stake_cred = caddr::StakeCredential::from_keyhash(&stake_cred_key);
    //let mut netbyte : u8 = 0b1111;
    //if *network == clib::NetworkIdKind::Testnet {
    //     netbyte = 0b1110;
    //}
    match hex::decode(str) {
        Ok(bytes) => Ok(caddr::Address::from_bytes(bytes)?),
        Err(_) => match caddr::Address::from_bech32(str) {
            Ok(addr) => Ok(addr),
            Err(_e) => Err(MurinError::new(
                "The provided Address is not byte encoded not bech32 encoded, Address invalid!",
            )),
        },
    }
}

/// decode a vector of hex encoded addresses and return a vector of deserialized addresses
pub async fn addresses_from_string(addresses: &Vec<String>) -> Result<Vec<caddr::Address>, MurinError> {
    let mut ret = Vec::<caddr::Address>::new();
    //Ok(caddr::Address::from_bytes(hex::decode(bytes)?)?)
    for addr in addresses {
        ret.push(b_decode_addr(addr).await?)
    }
    if !ret.is_empty() || addresses.is_empty() {
        Ok(ret)
    } else {
        Err(MurinError::new("ERROR: No valid Addresses provided"))
    }
}

/// convert hex encoded utxos into TransactionUnspentOutputs, filter collateral and excluded utxos if provided
pub async fn transaction_unspent_outputs_from_string_vec(
    enc_txuos: &[String],
    col_utxo: Option<&String>,
    enc_excl: Option<&Vec<String>>,
) -> Result<TransactionUnspentOutputs, MurinError> {
    let mut txuos = TransactionUnspentOutputs::new();
    let mut utxos = enc_txuos.to_vec();

    // Filter exculdes if there are some
    if enc_excl.is_some() {
        for excl in enc_excl.unwrap() {
            utxos.retain(|utxo| *utxo != *excl);
        }
    }
    // filter collateral if there is some
    if col_utxo.is_some() {
        utxos.retain(|utxo| *utxo != *col_utxo.unwrap());
    }
    // convert to TransactionunspentOutputs
    for utxo in utxos {
        txuos.add(&TransactionUnspentOutput::from_bytes(hex::decode(utxo)?)?);
    }
    Ok(txuos)
}

pub async fn get_transaction_unspent_output(
    encoded_utxo: &String,
) -> Result<TransactionUnspentOutput, MurinError> {
    Ok(TransactionUnspentOutput::from_bytes(hex::decode(
        encoded_utxo,
    )?)?)
}

/// converts network id into NetworkIdKind from ser.lib
pub async fn get_network_kind(net_id: u64) -> Result<clib::NetworkIdKind, MurinError> {
    match net_id {
        0 => Ok(clib::NetworkIdKind::Testnet),
        1 => Ok(clib::NetworkIdKind::Mainnet),
        _ => Err(MurinError::new("ERROR: Invalid network id provided")),
    }
}

pub fn get_stake_address(addr: &caddr::Address) -> Result<ccrypto::Ed25519KeyHash, MurinError> {
    debug!("Address in get_stake_address: {:?}", addr.to_bech32(None));

    match caddr::BaseAddress::from_address(addr) {
        Some(addr) => Ok(addr
            .stake_cred()
            .to_keyhash()
            .ok_or_else(|| MurinError::new("ERROR: cannot get key hash from stake credential"))?),
        None => match caddr::RewardAddress::from_address(addr) {
            Some(reward) => Ok(reward
                .payment_cred()
                .to_keyhash()
                .ok_or_else(|| MurinError::new("ERROR: cannot get keyhash from reward address"))?),
            None => {
                let enterprise_address = caddr::EnterpriseAddress::from_address(addr)
                    .ok_or_else(|| MurinError::new("ERROR: cannot decode Enterprise Address"))?;
                let payment_cred_key_ = enterprise_address.payment_cred();
                match payment_cred_key_.kind() {
                    caddr::StakeCredKind::Key => {
                        let cred_key_ = payment_cred_key_.to_keyhash().ok_or_else(|| {
                            MurinError::new("ERROR: cannot get key hash from stake credential")
                        })?;
                        let scripthash_bytes = cred_key_.to_bytes();
                        Ok(ccrypto::Ed25519KeyHash::from_bytes(scripthash_bytes)?)
                    }

                    caddr::StakeCredKind::Script => {
                        let cred_key_ = payment_cred_key_.to_scripthash().ok_or_else(|| {
                            MurinError::new("ERROR: cannot get key hash from stake credential")
                        })?;
                        let scripthash_bytes = cred_key_.to_bytes();
                        Ok(ccrypto::Ed25519KeyHash::from_bytes(scripthash_bytes)?)
                    }
                }
            }
        },
    }
}

pub fn get_reward_address(addr: &caddr::Address) -> Result<caddr::Address, MurinError> {
    match caddr::RewardAddress::from_address(addr) {
        Some(rwa) => Ok(rwa.to_address()),
        None => {
            if let Some(baddr) = BaseAddress::from_address(addr) {
                return Ok(
                    caddr::RewardAddress::new(addr.network_id()?, &baddr.stake_cred()).to_address(),
                );
            }
            if let Some(eaddr) = EnterpriseAddress::from_address(addr) {
                return Ok(
                    caddr::RewardAddress::new(addr.network_id()?, &eaddr.payment_cred())
                        .to_address(),
                );
            }

            Err(MurinError::new(
                "Error: Cannot retrieve Reward Address from given address",
            ))
        }
    }
}

pub fn get_bech32_stake_address_from_str(str: &str) -> Result<String, MurinError> {
    let address = match caddr::Address::from_bech32(str) {
        Ok(addr) => Ok(addr),
        Err(_e) => Err(MurinError::new(
            "The provided Address is not byte encoded not bech32 encoded, Address invalid!",
        )),
    }?;
    log::info!("String: {} , Address: {:?}", str, address);

    let network = address.network_id()?;
    match BaseAddress::from_address(&address) {
        Some(base) => Ok(caddr::RewardAddress::new(network, &base.stake_cred())
            .to_address()
            .to_bech32(None)?),
        None => Err(MurinError::new(
            "The Address does not contain stake credentials",
        )),
    }
}

pub fn get_pubkey(addr: &caddr::Address) -> Result<ccrypto::Ed25519KeyHash, MurinError> {
    //info!("\nAddress in get_payment_address: {:?}",addr);
    let address = caddr::BaseAddress::from_address(addr);
    let err = MurinError::new("ERROR wallet::get_pubkey gut not deserialize pub key from address");
    match address {
        Some(base_addr) => base_addr.payment_cred().to_keyhash().ok_or(err),
        None => {
            let enterprise_address = caddr::EnterpriseAddress::from_address(addr).ok_or(&err);
            if let Ok(payment_cred_key) = enterprise_address {
                match payment_cred_key.payment_cred().kind() {
                    caddr::StakeCredKind::Key => {
                        if let Ok(cred_key_) =
                            payment_cred_key.payment_cred().to_keyhash().ok_or(&err)
                        {
                            Ok(ccrypto::Ed25519KeyHash::from_bytes(cred_key_.to_bytes())?)
                        } else {
                            Err(err.clone())
                        }
                    }

                    caddr::StakeCredKind::Script => {
                        if let Ok(cred_key_) =
                            payment_cred_key.payment_cred().to_scripthash().ok_or(&err)
                        {
                            Ok(ccrypto::Ed25519KeyHash::from_bytes(cred_key_.to_bytes())?)
                        } else {
                            Err(err.clone())
                        }
                    }
                }
            } else {
                Err(err)
            }
        }
    }
}

pub fn create_wallet() -> (
    clib::crypto::Bip32PrivateKey,
    PrivateKey,
    PublicKey,
    Ed25519KeyHash,
    String,
    String,
) {
    let root_key1: clib::crypto::Bip32PrivateKey =
        clib::crypto::Bip32PrivateKey::generate_ed25519_bip32().unwrap();
    let account_key1 = root_key1
        .derive(crate::txbuilder::harden(1852u32))
        .derive(crate::txbuilder::harden(1815u32))
        .derive(crate::txbuilder::harden(0u32));
    let ac1_chaincode = account_key1.chaincode();
    let ac1_private_key = account_key1.to_raw_key(); // for signatures
    let ac1_public_key = account_key1.to_raw_key().to_public();
    let ac1_public_key_hash = account_key1.to_raw_key().to_public().hash(); // for Native Script Input / Verification
    let vkey1 = "5840".to_string()
        + &((hex::encode(ac1_public_key.as_bytes())) + &hex::encode(ac1_chaincode.clone())); // .vkey
    let skey1 = "5880".to_string()
        + &(hex::encode(ac1_private_key.as_bytes())
            + &hex::encode(ac1_public_key.as_bytes())
            + &hex::encode(ac1_chaincode)); // .skey
    (
        root_key1,
        ac1_private_key,
        ac1_public_key,
        ac1_public_key_hash,
        vkey1,
        skey1,
    )
}

pub fn create_drslkeypair() -> (String, String, String) {
    let root_key1: clib::crypto::Bip32PrivateKey =
        clib::crypto::Bip32PrivateKey::generate_ed25519_bip32().unwrap();
    let account_key1 = root_key1
        .derive(crate::txbuilder::harden(2733u32))
        .derive(crate::txbuilder::harden(2778u32))
        .derive(crate::txbuilder::harden(0u32));
    let ac1_private_key = account_key1.to_raw_key(); // for signatures
    let ac1_public_key = account_key1.to_raw_key().to_public();
    let ac1_public_key_hash = account_key1.to_raw_key().to_public().hash(); // for Native Script Input / Verification
    (
        ac1_private_key.to_bech32(),
        ac1_public_key.to_bech32(),
        hex::encode(ac1_public_key_hash.to_bytes()),
    )
}

pub fn create_bip0039_wallet(
    password: Option<String>,
) -> Result<(String, Vec<String>), MurinError> {
    let mut entropy = [0u8; 32];
    OsRng.fill_bytes(&mut entropy);
    let mnemonic = bip39::Mnemonic::from_entropy(&entropy)?;

    let seed = if let Some(password) = password.clone() {
        mnemonic.to_seed_normalized(&password)
    } else {
        mnemonic.to_seed_normalized("")
    };

    let words = mnemonic
        .word_iter()
        .fold(Vec::<String>::new(), |mut acc, n| {
            acc.push(n.to_string());
            acc
        });

    let root_key: clib::crypto::Bip32PrivateKey =
        clib::crypto::Bip32PrivateKey::from_bip39_entropy(&seed, {
            if password.is_some() {
                password.as_ref().unwrap().as_bytes()
            } else {
                "".as_bytes()
            }
        });

    /*
       let account_key1 = root_key
           .derive(crate::txbuilders::harden(1852u32))
           .derive(crate::txbuilders::harden(1815u32))
           .derive(crate::txbuilders::harden(0u32));
       let ac1_chaincode = account_key1.chaincode();
       let ac1_private_key = account_key1.to_raw_key(); // for signatures
       let ac1_public_key = account_key1.to_raw_key().to_public();
       let ac1_public_key_hash = account_key1.to_raw_key().to_public().hash(); // for Native Script Input / Verification
       let vkey1 = "5840".to_string()
           + &((hex::encode(ac1_public_key.as_bytes())) + &hex::encode(ac1_chaincode.clone())); // .vkey
       let skey1 = "5880".to_string()
           + &(hex::encode(ac1_private_key.as_bytes())
               + &hex::encode(ac1_public_key.as_bytes())
               + &hex::encode(ac1_chaincode)); // .skey
    */
    Ok((root_key.to_hex(), words))
}

pub fn restore_bip0039_wallet(
    words: Vec<String>,
    password: Option<String>,
) -> Result<(String, ccrypto::Bip32PrivateKey), MurinError> {
    let mut entropy = [0u8; 32];
    OsRng.fill_bytes(&mut entropy);

    let s = words.iter().fold(String::new(), |mut acc, n| {
        acc.push_str(n);
        acc.push(' ');
        acc
    });

    let mnemonic = bip39::Mnemonic::parse(std::borrow::Cow::from(s)).unwrap();

    let seed = if let Some(password) = password.clone() {
        mnemonic.to_seed_normalized(&password)
    } else {
        mnemonic.to_seed_normalized("")
    };

    let root_key: clib::crypto::Bip32PrivateKey =
        clib::crypto::Bip32PrivateKey::from_bip39_entropy(&seed, {
            if password.is_some() {
                password.as_ref().unwrap().as_bytes()
            } else {
                "".as_bytes()
            }
        });

    /*
       let account_key1 = root_key
           .derive(crate::txbuilders::harden(1852u32))
           .derive(crate::txbuilders::harden(1815u32))
           .derive(crate::txbuilders::harden(0u32));
       let ac1_chaincode = account_key1.chaincode();
       let ac1_private_key = account_key1.to_raw_key(); // for signatures
       let ac1_public_key = account_key1.to_raw_key().to_public();
       let ac1_public_key_hash = account_key1.to_raw_key().to_public().hash(); // for Native Script Input / Verification
       let vkey1 = "5840".to_string()
           + &((hex::encode(ac1_public_key.as_bytes())) + &hex::encode(ac1_chaincode.clone())); // .vkey
       let skey1 = "5880".to_string()
           + &(hex::encode(ac1_private_key.as_bytes())
               + &hex::encode(ac1_public_key.as_bytes())
               + &hex::encode(ac1_chaincode)); // .skey
    */
    Ok((root_key.to_hex(), root_key))
}

#[cfg(test)]
mod tests {

    use crate::{create_bip0039_wallet, restore_bip0039_wallet};

    #[tokio::test]
    async fn bip0039_seed_test() {
        let wallet = create_bip0039_wallet(Some("otto".to_string())).unwrap();
        println!("{:?}", wallet.1);
        println!("{:?}", wallet.0);
        let restored = restore_bip0039_wallet(wallet.1, None).unwrap();
        println!("{:?}", restored.0);
        assert_ne!(wallet.0, restored.0)
    }

    #[tokio::test]
    async fn bip0039_restore_test() {
        let wallet = vec![
            "already", "ivory", "floor", "demise", "turn", "nurse", "code", "cage", "hobby",
            "transfer", "struggle", "enough", "topple", "citizen", "wasp", "amateur", "vacuum",
            "banner", "resist", "cupboard", "delay", "area", "dry", "silly",
        ];
        let mut wal = Vec::<String>::new();
        wallet.iter().for_each(|n| wal.push(n.to_string()));
        let _skey = "10e892baf1a2dc4a14d90a0660a3f8155c1429ac4cb611f43f882c8c4bf2104fd2bdbccfd238068e36c2666fe2ca6563e0ebf4082d710f8a1367b5b2c95875b697ea593ac107552864a0ccd285d4d0cbbcfeee44200ba68028d34fbeaf050417";
        let restored = restore_bip0039_wallet(wal, Some("Otto".to_string())).unwrap();
        //assert_eq!(skey, restored.0);

        let stake_key = restored
            .1
            .derive(crate::txbuilder::harden(1852u32))
            .derive(crate::txbuilder::harden(1815u32))
            .derive(crate::txbuilder::harden(0u32))
            .derive(2)
            .derive(0);
        let stake_key_hash = stake_key.to_raw_key().to_public().hash();

        let account_key1 = restored
            .1
            .derive(crate::txbuilder::harden(1852))
            .derive(crate::txbuilder::harden(1815))
            .derive(crate::txbuilder::harden(0))
            .derive(0)
            .derive(1);
        let ac1_chaincode = account_key1.chaincode();
        let ac1_private_key = account_key1.to_raw_key(); // for signatures
        let ac1_public_key = account_key1.to_raw_key().to_public();
        let ac1_public_key_hash = account_key1.to_raw_key().to_public().hash(); // for Native Script Input / Verification
        let _vkey1 = "5840".to_string()
            + &((hex::encode(ac1_public_key.as_bytes())) + &hex::encode(ac1_chaincode.clone())); // .vkey
        let _skey1 = "5880".to_string()
            + &(hex::encode(ac1_private_key.as_bytes())
                + &hex::encode(ac1_public_key.as_bytes())
                + &hex::encode(ac1_chaincode)); // .skey
        let pycr = crate::clib::address::StakeCredential::from_keyhash(&ac1_public_key_hash);
        let stkcr = crate::clib::address::StakeCredential::from_keyhash(&stake_key_hash);
        let addr = crate::clib::address::BaseAddress::new(1, &pycr, &stkcr).to_address();
        let addr2 = crate::clib::address::EnterpriseAddress::new(1, &pycr).to_address();
        println!("Base: {}", addr.to_bech32(None).unwrap());
        println!("Enterprise: {}", addr2.to_bech32(None).unwrap());
    }
}
