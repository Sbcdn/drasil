use hugin::error::SystemDBError;
/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use murin::clib::{crypto::Bip32PrivateKey, NativeScript, NativeScripts, ScriptAll, ScriptPubkey};
use murin::*;

use hugin::database::drasildb::{TBContracts, TBMultiSigLoc};
use structopt::StructOpt;

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

    #[structopt(short, long, about = "version number as float")]
    version: f32,

    #[structopt(short, long, about = "wallet for fee distribution")]
    wallet: Option<String>,

    #[structopt(
        short,
        long,
        about = "Optional fixed fee for a transaction for using the service in lovelace"
    )]
    fee: Option<i64>,
}
pub fn harden(num: u32) -> u32 {
    0x80000000 + num
}

#[tokio::main]
async fn main() -> Result<(), SystemDBError> {
    let opt = Opt::from_args();

    //let mut network = NetworkIdKind::Mainnet;
    //let mut prefix = "addr";
    let mut net_bytes = 0b0001;
    if opt.testnet.is_some() {
        //  network = NetworkIdKind::Testnet;
        //  prefix = "addr_test";
        println!("Got testnet");
        net_bytes = 0b0000;
    }

    // create key1
    let root_key1 = Bip32PrivateKey::generate_ed25519_bip32()?;
    let pvk1_root_bytes = hex::encode(root_key1.as_bytes());

    let account_key1 = root_key1
        .derive(harden(1852u32))
        .derive(harden(1815u32))
        .derive(harden(0u32));
    let ac1_chaincode = account_key1.chaincode();

    let ac1_private_key = account_key1.to_raw_key(); // for signatures

    let ac1_publick_key = account_key1.to_raw_key().to_public();
    let ac1_public_key_hash = account_key1.to_raw_key().to_public().hash(); // for Native Script Input / Verification
                                                                            // let utxo_pub_key1 = account_key1.derive(0).derive(0).to_public(); // unclear
    let _vkey1 = "5840".to_string()
        + &((hex::encode(ac1_publick_key.as_bytes())) + &hex::encode(ac1_chaincode.clone())); // .vkey
    let _skey1 = "5880".to_string()
        + &(hex::encode(ac1_private_key.as_bytes())
            + &hex::encode(ac1_publick_key.as_bytes())
            + &hex::encode(ac1_chaincode)); // .vkey

    // create key2
    let root_key2 = Bip32PrivateKey::generate_ed25519_bip32()?;
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

    /*
    // Signing Key (skey)
    {
    "type": "PaymentExtendedSigningKeyShelley_ed25519_bip32",
    "description": "",
    "cborHex": "5880b0bf46232c7f0f58ad333030e43ffbea7c2bb6f8135bd05fb0d343ade8453c5eacc7ac09f77e16b635832522107eaa9f56db88c615f537aa6025e6c23da98ae8fbbbf6410e24532f35e9279febb085d2cc05b3b2ada1df77ea1951eb694f3834b0be1868d1c36ef9089b3b094f5fe1d783e4d5fea14e2034c0397bee50e65a1a"
    }

    // Verification Key (vkey)
    $ cardano-cli key verification-key --signing-key-file key.skey --verification-key-file key.vkey
    {
    "type": "PaymentExtendedVerificationKeyShelley_ed25519_bip32",
    "description": "",
    "cborHex": "5840fbbbf6410e24532f35e9279febb085d2cc05b3b2ada1df77ea1951eb694f3834b0be1868d1c36ef9089b3b094f5fe1d783e4d5fea14e2034c0397bee50e65a1a"
    }

    */

    let mut native_scripts = NativeScripts::new();
    native_scripts.add(&NativeScript::new_script_pubkey(&ScriptPubkey::new(
        &ac1_public_key_hash,
    )));
    native_scripts.add(&NativeScript::new_script_pubkey(&ScriptPubkey::new(
        &ac2_public_key_hash,
    )));

    let rwd_script = NativeScript::new_script_all(&ScriptAll::new(&native_scripts));
    let script_hash = rwd_script.hash(); //policyId

    let stake_creds = clib::address::StakeCredential::from_scripthash(&script_hash);
    let script_address_e =
        clib::address::EnterpriseAddress::new(net_bytes, &stake_creds).to_address();

    let d = &format!(
        "RWD Multi Signature Native Script user: {:?}",
        opt.user.clone()
    )[..];
    let description = Some(d);

    let contract_type = "sporwc";
    let _ = TBContracts::create_contract(
        &opt.user,
        &opt.contract_id,
        contract_type,
        description,
        &opt.version,
        &hex::encode(rwd_script.to_bytes()),
        &script_address_e.to_bech32(None)?,
        Some(&hex::encode(script_hash.to_bytes())),
        &false,
    )?;

    //println!("Required Signers: {:?}",rwd_script.get_required_signers());
    for i in 0..rwd_script.get_required_signers().len() {
        println!(
            "Required Signer {}: {:?}",
            i,
            hex::encode(rwd_script.get_required_signers().get(i).to_bytes())
        );
    }

    let pvks = vec![pvk1_root_bytes, pvk2_root_bytes];

    let _ = TBMultiSigLoc::create_multisig_keyloc(
        &opt.user,
        &opt.contract_id,
        &opt.version,
        &script_address_e.to_bech32(None)?,
        opt.wallet.as_ref(),
        opt.fee.as_ref(),
        &pvks,
        &false,
    )
    .await?;

    if opt.output.is_some() {
        println!("Script Hash: {:?}", hex::encode(script_hash.to_bytes()));
        println!("Script Address: {:?}", script_address_e.to_bech32(None)?);
        println!("Native Script: {:?}", hex::encode(rwd_script.to_bytes()));
    }

    Ok(())
}
