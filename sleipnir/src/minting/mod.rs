/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
pub use crate::error::SleipnirError;

use chrono::{DateTime, Utc};
use hugin::database::*;
use murin::*;

pub async fn create_policy_script(
    network: murin::clib::NetworkIdKind,
    user_id: i64,
    mint_signer: Option<clib::address::Address>,
    time_constraint: Option<DateTime<Utc>>,
    //contract_type : String,
) -> Result<i64, SleipnirError> {
    // create rwd wallet  & contract

    let mut net_bytes = 0b0001;
    if network == murin::clib::NetworkIdKind::Testnet {
        net_bytes = 0b0000;
    }

    // create key1
    let root_key1: clib::crypto::Bip32PrivateKey =
        clib::crypto::Bip32PrivateKey::generate_ed25519_bip32()?;
    let pvk1_root_bytes = hex::encode(root_key1.as_bytes());
    let account_key1 = root_key1
        .derive(harden(1852u32))
        .derive(harden(1815u32))
        .derive(harden(0u32));
    let ac1_chaincode = account_key1.chaincode();
    let ac1_private_key = account_key1.to_raw_key(); // for signatures
    let ac1_publick_key = account_key1.to_raw_key().to_public();
    let ac1_public_key_hash = account_key1.to_raw_key().to_public().hash(); // for Native Script Input / Verification
    let _vkey1 = "5840".to_string()
        + &((hex::encode(ac1_publick_key.as_bytes())) + &hex::encode(ac1_chaincode.clone())); // .vkey
    let _skey1 = "5880".to_string()
        + &(hex::encode(ac1_private_key.as_bytes())
            + &hex::encode(ac1_publick_key.as_bytes())
            + &hex::encode(ac1_chaincode)); // .vkey

    // create key2
    let root_key2: clib::crypto::Bip32PrivateKey =
        clib::crypto::Bip32PrivateKey::generate_ed25519_bip32()?;
    let pvk2_root_bytes = hex::encode(root_key2.as_bytes());
    let account_key2 = root_key2
        .derive(harden(1852u32))
        .derive(harden(1815u32))
        .derive(harden(0u32));
    let ac2_chaincode = account_key2.chaincode();
    let ac2_private_key = account_key2.to_raw_key(); // for signatures
    let ac2_publick_key = account_key2.to_raw_key().to_public();
    let ac2_public_key_hash = account_key2.to_raw_key().to_public().hash(); // for Native Script Input / Verification
    let _vkey2 = "5840".to_string()
        + &((hex::encode(ac2_publick_key.as_bytes())) + &hex::encode(ac2_chaincode.clone())); // .vkey
    let _skey2 = "5880".to_string()
        + &(hex::encode(ac2_private_key.as_bytes())
            + &hex::encode(ac2_publick_key.as_bytes())
            + &hex::encode(ac2_chaincode)); // .vkey

    let mut native_scripts = NativeScripts::new();
    native_scripts.add(&NativeScript::new_script_pubkey(&ScriptPubkey::new(
        &ac1_public_key_hash,
    )));
    native_scripts.add(&NativeScript::new_script_pubkey(&ScriptPubkey::new(
        &ac2_public_key_hash,
    )));

    if let Some(addr) = mint_signer.clone() {
        match clib::address::BaseAddress::from_address(&addr) {
            Some(baddr) => {
                native_scripts.add(&NativeScript::new_script_pubkey(&ScriptPubkey::new(
                    &baddr.payment_cred().to_keyhash().unwrap(),
                )));
            }
            None => {
                return Err(SleipnirError::new(
                    "Provided Wallet Signature Address is invalid",
                ));
            }
        }
    };

    if let Some(date) = time_constraint {
        let slot = murin::minter::calculate_slot_from_date(date) as u64;
        native_scripts.add(&NativeScript::new_timelock_expiry(
            &clib::TimelockExpiry::new_timelockexpiry(&murin::clib::utils::to_bignum(slot)),
        ));
    }

    let mint_script = NativeScript::new_script_all(&ScriptAll::new(&native_scripts));
    let policy_id = mint_script.hash(); //policyId

    let stake_creds = clib::address::StakeCredential::from_scripthash(&policy_id);
    let script_address_e =
        clib::address::EnterpriseAddress::new(net_bytes, &stake_creds).to_address();
    let sc_address_bech32 = script_address_e.to_bech32(None)?;
    let d = &format!("PolicyID:{}", hex::encode(policy_id.to_bytes()))[..];
    let description = Some(d);

    let contract_id = TBContracts::get_next_contract_id(&user_id)?;
    let contract_type = "mint"; // &contract_type; //"mintft" || "mintnft" ;
    let _ = TBContracts::create_contract(
        &user_id,
        &contract_id,
        contract_type,
        description,
        &1.0,
        &hex::encode(mint_script.to_bytes()),
        &sc_address_bech32,
        Some(&hex::encode(policy_id.to_bytes())),
        &false,
    )?;

    let pvks = vec![pvk1_root_bytes, pvk2_root_bytes];

    let mut signer = None;
    if let Some(signaddr) = mint_signer {
        signer = Some(signaddr.to_bech32(None)?);
    }

    let _ = TBMultiSigLoc::create_multisig_keyloc(
        &user_id,
        &contract_id,
        &1.0,
        &sc_address_bech32,
        signer.as_ref(),
        None,
        &pvks,
        &false,
    )
    .await?;

    Ok(contract_id)
}
