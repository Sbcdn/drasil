use crate::htypes::*;
use crate::MurinError;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{
    address as caddr, crypto as ccrypto, plutus, tx_builder as ctxb, utils as cutils,
};
use clib::address::BaseAddress;
use clib::address::EnterpriseAddress;
use clib::address::StakeCredKind;
use std::io::{self, BufRead};

use crate::txbuilders;
use cryptoxide::blake2b::Blake2b;
use std::env;
use std::str;
//use emurgo_message_signing::{ utils::FromBytes as msfrom_bytes, utils::ToBytes as msto_bytes};
//use emurgo_message_signing as ms;

pub(crate) fn blake2b256(data: &[u8]) -> [u8; 32] {
    let mut out = [0; 32];
    Blake2b::blake2b(&mut out, data, &[]);
    out
}

pub fn _ccli_query_utxos_address(
    addr: &caddr::Address,
    network: String,
) -> Option<TransactionUnspentOutputs> {
    let cardano_cli_env = env::var("CARDANO_CLI_PATH");
    if cardano_cli_env.is_err() {
        panic!("CARDANO_CLI_PATH not set");
    }
    let cardano_cli_path = cardano_cli_env.unwrap();

    let mut txuos = TransactionUnspentOutputs::new();

    let net = get_network(&network);
    let mut magic = "--mainnet";
    let mut magic_n = "";
    if net.0 == clib::NetworkIdKind::Testnet {
        magic = "--testnet-magic";
        magic_n = "1097911063";
    }

    let ret = std::process::Command::new(cardano_cli_path)
        .arg("query")
        .arg("utxo")
        .arg("--address")
        .arg(addr.to_bech32(Some(net.1.to_string())).unwrap())
        .arg(magic)
        .arg(magic_n)
        //.stdout(std::process::Stdio::null())
        //.stderr(std::process::Stdio::piped())
        .output()
        .expect("Get utxos with Cardano-Cli failed");

    if ret.status.success() {
        let cursor = io::Cursor::new(ret.stdout);
        let lines_iter = cursor.lines().map(|l| l.unwrap());

        for line in lines_iter {
            if &line[0..1] != " " && &line[0..1] != "-" {
                let mut cols = line.split_whitespace();
                let tx_hash: ccrypto::TransactionHash = match cols.next() {
                    Some(hash) => {
                        ccrypto::TransactionHash::from_bytes(hex::decode(hash).unwrap()).unwrap()
                    }
                    None => {
                        panic!("TxHash expected!")
                    }
                };
                let tx_index: u32 = match cols.next() {
                    Some(index) => index.parse::<u32>().unwrap(),
                    None => {
                        panic!("TxIndex (u32) expected!")
                    }
                };
                let lovelace: cutils::BigNum = match cols.next() {
                    Some(amount) => cutils::BigNum::from_str(amount).unwrap(),
                    None => {
                        panic!("Lovelace value expected!")
                    }
                };
                let _l_unit = cols.next().unwrap();

                let mut val = cutils::Value::new(&lovelace);

                let mut ma = clib::MultiAsset::new();
                let mut datumhash: Option<ccrypto::DataHash> = None;
                let mut amount: u64 = 0;
                let mut next = "";

                'l1: loop {
                    match cols.next() {
                        Some(elem) => match elem {
                            "+" => {
                                continue 'l1;
                            }
                            "TxOutDatumNone" => {
                                continue 'l1;
                            }
                            _ => match elem.parse::<u64>() {
                                Ok(num) => {
                                    amount = num;
                                    next = "token"
                                }
                                Err(_) => {
                                    if next == "token" {
                                        let mut t = elem.split('.');
                                        let mut asset = clib::Assets::new();
                                        let policy = clib::PolicyID::from_bytes(
                                            hex::decode(t.next().unwrap()).unwrap(),
                                        )
                                        .unwrap();
                                        let asset_name = clib::AssetName::new(
                                            hex::decode(t.next().unwrap()).unwrap(),
                                        )
                                        .unwrap();
                                        asset.insert(&asset_name, &cutils::to_bignum(amount));

                                        match ma.get(&policy) {
                                            Some(p) => match p.get(&asset_name) {
                                                Some(a) => {
                                                    ma.get(&policy).unwrap().insert(
                                                        &asset_name,
                                                        &a.checked_add(&cutils::to_bignum(amount))
                                                            .unwrap(),
                                                    );
                                                }
                                                None => {
                                                    ma.insert(&policy, &asset);
                                                }
                                            },
                                            None => {
                                                ma.insert(&policy, &asset);
                                            }
                                        }
                                        next = "";
                                        amount = 0;
                                    } else {
                                        match ccrypto::DataHash::from_bytes(
                                            hex::decode(elem).unwrap(),
                                        ) {
                                            Ok(dh) => {
                                                datumhash = Some(dh);
                                            }
                                            Err(_) => datumhash = None,
                                        }
                                    }
                                }
                            },
                        },

                        None => {
                            break 'l1;
                        }
                    }
                }

                if ma.len() > 0 {
                    val.set_multiasset(&ma);
                }

                let mut output = clib::TransactionOutput::new(addr, &val);
                if let Some(dh) = datumhash {
                    output.set_data_hash(&dh);
                }

                let txuo = TransactionUnspentOutput::new(
                    &clib::TransactionInput::new(&tx_hash, tx_index),
                    &output,
                );
                txuos.add(&txuo);
            }
        }
        //    io::stdin().read_line(&mut line).expect("ERROR: Could not read Tx-Data from stdin");
        //    info!("Line: {}",line);
        //let txh = TxHash {tx_hash : tx_hash.clone(), message : "success".to_string()};
        //serde_json::to_writer(&io::stdout(),&txh).unwrap();
        //let _r = fs::remove_file(format!("tmp_tx/{}.tx",tx_hash));
        return Some(txuos);
    } else {
        info!("Error: could not read stdin, {:?}", ret.status);
    }

    None
}

pub fn get_nfts_for_sale(
    token_utxos: &TransactionUnspentOutputs,
) -> Vec<(ccrypto::ScriptHash, clib::AssetName, cutils::BigNum)> {
    let mut ret = Vec::<(ccrypto::ScriptHash, clib::AssetName, cutils::BigNum)>::new();
    for i in 0..token_utxos.len() {
        let value = token_utxos.get(i).output().amount();
        for p in 0..value.multiasset().unwrap().keys().len() {
            let policy_id = value.multiasset().unwrap().keys().get(p);
            let assets = value.multiasset().unwrap().get(&policy_id).unwrap();
            for a in 0..assets.len() {
                let tn = assets.keys().get(a);
                ret.push((policy_id.clone(), tn.clone(), assets.get(&tn).unwrap()))
            }
        }
    }

    ret
}

pub fn find_token_in_utxo(
    utxo: &TransactionUnspentOutput,
    cs: &ccrypto::ScriptHash,
    tn: &clib::AssetName,
) -> Option<usize> {
    let multi = utxo.output().amount().multiasset();
    if let Some(multi) = multi {
        for _ in 0..multi.keys().len() {
            match multi.get(cs) {
                Some(assets) => {
                    for j in 0..assets.len() {
                        match assets.get(tn) {
                            Some(_) => return Some(j),
                            None => continue,
                        }
                    }
                }
                None => continue,
            }
        }
    }
    None
}

pub fn get_token_amount(v: &cutils::Value) -> usize {
    let mut k = 0;
    match v.multiasset() {
        Some(multis) => {
            if multis.keys().len() > 0 {
                for i in 0..multis.keys().len() {
                    match multis.get(&multis.keys().get(i)) {
                        Some(assets) => {
                            k += assets.len();
                        }
                        None => continue,
                    }
                }
                k
            } else {
                0
            }
        }
        None => 0,
    }
}

pub fn get_ttl_tx(net: &cardano_serialization_lib::NetworkIdKind) -> u64 {
    if *net == cardano_serialization_lib::NetworkIdKind::Testnet {
        1800
    } else {
        7200
    }
}

pub fn _query_utxos_by_address_from_cli(
    addresses: Vec<caddr::Address>,
) -> TransactionUnspentOutputs {
    let mut txuos = TransactionUnspentOutputs::new();

    for i in 0..addresses.len() {
        let addr = addresses.get(i).unwrap();
        if let Some(utxos) = _ccli_query_utxos_address(addr, "testnet".to_string()) {
            txuos.merge(utxos)
        }
    }
    debug!("My Utxos: \n{:?}\n", txuos);
    txuos
}

//Todo: Return Result
pub fn get_smart_contract(
    sc: &Option<String>,
) -> Result<cardano_serialization_lib::plutus::PlutusScript, MurinError> {
    match sc {
        Some(sc) => {
            //let sc_data=fs::read_to_string(path).expect("ERROR: Could not read Tx-Data file");
            //let sc : SmartContract = serde_json::from_str(&sc_data).unwrap();
            //smart_contract = plutus::PlutusScript::new(hex::decode(sc.cborHex).unwrap());
            Ok(plutus::PlutusScript::new(hex::decode(sc)?))
        }
        None => {
            //
            Err(MurinError::new(&format!("Smart Contract not valid {sc:?}")))
        }
    }
}

pub fn get_network(nws: &String) -> (clib::NetworkIdKind, &str) {
    if nws == "testnet" {
        (clib::NetworkIdKind::Testnet, "addr_test")
    } else {
        (clib::NetworkIdKind::Mainnet, "addr")
    }
}

pub fn get_network_from_address(address: &String) -> Result<clib::NetworkIdKind, MurinError> {
    let addr: caddr::Address = crate::cip30::wallet::b_decode_addr_na(address)?;
    match addr.network_id()? {
        1 => Ok(clib::NetworkIdKind::Mainnet),
        _ => Ok(clib::NetworkIdKind::Testnet),
    }
}

pub fn get_input_position(
    inputs: clib::TransactionInputs,
    elem: TransactionUnspentOutput,
) -> (usize, Vec<ccrypto::TransactionHash>) {
    let mut index: usize;
    let mut my_index = Vec::<ccrypto::TransactionHash>::new();
    for i in 0..inputs.len() {
        debug!("Script Input: {:?} at position : {:?}\n", inputs.get(i), i);
        my_index.push(inputs.get(i).transaction_id());
        if inputs.get(i).transaction_id() == elem.input().transaction_id()
            && inputs.get(i).index() == elem.input().index()
        {
            index = i;
            debug!(
                "Found Script Input: {:?} at position : {:?}\n",
                inputs.get(i),
                index
            );
        }
    }

    debug!("\nUnsortiert: {:?}", my_index);
    my_index.sort();
    debug!("\nSortiert: {:?}", my_index);
    let index = my_index
        .iter()
        .enumerate()
        .find(|&r| r.1 == &elem.input().transaction_id())
        .unwrap()
        .0;
    debug!("\nIndex: {:?}\n", index);

    (index, my_index)
}

pub fn get_vkey_count(
    txuos: &TransactionUnspentOutputs,
    col: Option<&TransactionUnspentOutput>,
) -> usize {
    // Check for Number of Vkeys in the signature
    let mut vkey_counter = 0usize;
    let mut addresses = Vec::<std::vec::Vec<u8>>::new();
    for txi in 0..txuos.len() {
        if !addresses.contains(&txuos.get(txi).output().address().to_bytes()) {
            vkey_counter += 1;
            addresses.push(txuos.get(txi).output().address().to_bytes());
        }
    }
    if let Some(c) = col {
        if !txuos.contains_address(c.output().address()) {
            vkey_counter += 1;
        }
    }
    debug!(
        "Addresses: {:?}, InputTxUO Len: {:?}\n",
        addresses,
        txuos.len()
    );
    debug!("\n\nVkey Counter in Method: {:?}\n", vkey_counter);
    vkey_counter
}

pub fn make_dummy_vkeywitnesses(vkey_count: usize) -> ccrypto::Vkeywitnesses {
    let mut dummy_vkeywitnesses = ccrypto::Vkeywitnesses::new();
    let vkeywitness =
        ccrypto::Vkeywitness::from_bytes(hex::decode(DUMMY_VKEYWITNESS).unwrap()).unwrap();
    info!("Dummy Vkey Count: {:?}", vkey_count);
    for _ in 0..vkey_count {
        dummy_vkeywitnesses.add(&vkeywitness);
    }
    debug!(
        "\n\nVkeywitness: {:?}\n\n",
        hex::encode(vkeywitness.to_bytes())
    );
    dummy_vkeywitnesses
}

pub fn get_stake_address(addr: &caddr::Address) -> ccrypto::Ed25519KeyHash {
    debug!(
        "Address in get_stake_address: {:?}",
        hex::encode(addr.to_bytes())
    );
    let address = caddr::BaseAddress::from_address(addr);
    let stake_cred_key: ccrypto::Ed25519KeyHash;
    match address {
        Some(addr) => {
            stake_cred_key = addr.stake_cred().to_keyhash().unwrap();
        }
        None => {
            let enterprise_address = caddr::EnterpriseAddress::from_address(addr).unwrap();
            let payment_cred_key_ = enterprise_address.payment_cred();
            match payment_cred_key_.kind() {
                caddr::StakeCredKind::Key => {
                    let cred_key_ = payment_cred_key_.to_keyhash().unwrap();
                    let scripthash_bytes = cred_key_.to_bytes();
                    stake_cred_key = ccrypto::Ed25519KeyHash::from_bytes(scripthash_bytes).unwrap();
                }

                caddr::StakeCredKind::Script => {
                    let cred_key_ = payment_cred_key_.to_scripthash().unwrap();
                    let scripthash_bytes = cred_key_.to_bytes();
                    stake_cred_key = ccrypto::Ed25519KeyHash::from_bytes(scripthash_bytes).unwrap();
                }
            }
        }
    }
    stake_cred_key
}

pub fn get_payment_address(addr: &caddr::Address) -> ccrypto::Ed25519KeyHash {
    //info!("\nAddress in get_payment_address: {:?}",addr);
    let address = caddr::BaseAddress::from_address(addr);
    let payment_cred_key: ccrypto::Ed25519KeyHash;
    match address {
        Some(base_addr) => {
            payment_cred_key = base_addr.payment_cred().to_keyhash().unwrap();
        }
        None => {
            let enterprise_address = caddr::EnterpriseAddress::from_address(addr).unwrap();
            let payment_cred_key_ = enterprise_address.payment_cred();
            match payment_cred_key_.kind() {
                caddr::StakeCredKind::Key => {
                    let cred_key_ = payment_cred_key_.to_keyhash().unwrap();
                    let scripthash_bytes = cred_key_.to_bytes();
                    payment_cred_key =
                        ccrypto::Ed25519KeyHash::from_bytes(scripthash_bytes).unwrap();
                }

                caddr::StakeCredKind::Script => {
                    let cred_key_ = payment_cred_key_.to_scripthash().unwrap();
                    let scripthash_bytes = cred_key_.to_bytes();
                    payment_cred_key =
                        ccrypto::Ed25519KeyHash::from_bytes(scripthash_bytes).unwrap();
                }
            }
        }
    }
    //info!("Payment Addres: {:?}\n",hex::encode(payment_cred_key.to_bytes()));
    payment_cred_key
}
/*
pub fn make_cardano_cli_tx(tx: String, tx_hash: String, submit: String, node_ok: bool) {
    let cli_tx = SmartContract {
        r#type: "Tx AlonzoEra".to_string(),
        description: "artifct CLI transaction".to_string(),
        cborHex: tx,
    };
    //serde_json::to_writer(&File::create("tx_final.artifct").unwrap(),&cli_tx).unwrap();
    //info!("Create Cardano-Cli Transaction File: tx_final.artifct");

    info!("Tx_Hash: {:?}", tx_hash);
    if submit != "false" && node_ok {
        let cardano_cli_env = env::var("CARDANO_CLI_PATH");
        if cardano_cli_env.is_err() {
            panic!("CARDANO_CLI_PATH not set");
        }
        let cardano_cli_path = cardano_cli_env.unwrap();

        let tmp_dir = TempDir::new("tmp_tx").unwrap();
        let file_path = tmp_dir.path().join(tx_hash.clone());
        let file = File::create(file_path.clone()).unwrap();
        //let file2 = File::create("tx.tx").unwrap();
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &cli_tx).unwrap();
        writer.flush().unwrap();

        let ret = std::process::Command::new(cardano_cli_path)
            .arg("transaction")
            .arg("submit")
            .arg("--tx-file")
            .arg(file_path)
            .arg("--testnet-magic")
            .arg("1097911063")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .expect("Submit with Cardano-Cli failed");
        if ret.status.success() {
            let txh = TxHash {
                tx_hash: tx_hash.clone(),
                message: "success".to_string(),
            };
            serde_json::to_writer(&io::stdout(), &txh).unwrap();
            let _r = fs::remove_file(format!("tmp_tx/{}.tx", tx_hash));
        } else {
            let txh = TxHash {
                tx_hash,
                message: String::from_utf8(ret.stderr).unwrap(),
            };
            serde_json::to_writer(&io::stdout(), &txh).unwrap();
        }
    } else {
        serde_json::to_writer(&io::stdout(), &cli_tx).unwrap();
    }
}
 */
pub fn _api_sign_tx(message: clib::TransactionBody) -> ccrypto::Vkeywitness {
    // TODO Working with private keys
    let api_priv_key =
        ccrypto::PrivateKey::from_extended_bytes(&hex::decode("_API_SKEY").unwrap()).unwrap();
    let api_pub_key = api_priv_key.to_public();
    let api_pub_key_hash = hex::encode(api_pub_key.hash().to_bytes());

    let api_vkey = ccrypto::Vkey::new(&api_pub_key);

    let api_signature = api_priv_key.sign(&message.to_bytes());

    let api_vkeywitness = ccrypto::Vkeywitness::new(&api_vkey, &api_signature);

    debug!("Api Public Key Hash {:?}", api_pub_key_hash);

    api_vkeywitness
}

// Add Message Lib
/*
fn verify_wallet(signed_data : String, address : String, message : String) -> bool {

    let wallet_address = caddr::Address::from_bech32(&address).unwrap();
    let signing_base_address = caddr::BaseAddress::from_address(&wallet_address).unwrap();
    let stake_cred_key = signing_base_address.stake_cred().to_keyhash().unwrap();

    let raw =  ms::SignedMessage::from_bytes(hex::decode(signed_data.clone()).unwrap()).unwrap();

    let cosesig1 = raw.as_cose_sign1().unwrap();
    let key = cosesig1.headers().protected().deserialized_headers().keys().get(0);
    let s_addr = cosesig1.headers().protected().deserialized_headers().header(&key).unwrap().as_bytes().unwrap();
    let mut s_addr_stake = s_addr.clone();
    s_addr_stake.remove(0);

    let payload_to_verify = cosesig1.payload().unwrap();
    let check1 = payload_to_verify == hex::decode(message.clone()).unwrap() &&  (s_addr == wallet_address.to_bytes() || s_addr_stake == stake_cred_key.to_bytes()); //
    info!("Check1 passed: {:?}",check1);

    // Reconstruct the SigStructure object so we can verify the signature
    let sig_struct_reconstructed = cosesig1.signed_data(None, Some(hex::decode(message).unwrap())).unwrap().to_bytes();
    let sig = clib::crypto::Ed25519Signature::from_bytes(cosesig1.signature()).unwrap();
    debug!("\nSig Reconstructed: {:?}\n",sig_struct_reconstructed);
    debug!("\nSig: {:?}\n",sig.to_bytes());
    let pubkey = ccrypto::PublicKey::from_bytes(&cosesig1.headers().protected().deserialized_headers().key_id().unwrap()).unwrap();
    info!("Check2 passed: {:?}",pubkey.verify(&sig_struct_reconstructed, &sig));
    let check2 = pubkey.verify(&sig_struct_reconstructed, &sig);


    debug!("Verification passed: {:?}",check1 && check2);

    return check1 && check2
}
 */

pub fn tx_output_data(
    txbody: clib::TransactionBody,
    txwitness: clib::TransactionWitnessSet,
    aux_data: clib::metadata::AuxiliaryData,
    used_utxos: String,
    royalties: u64,
    internal: bool,
) -> Result<BuildOutput, MurinError> {
    // Build and encode transaction
    let transaction = clib::Transaction::new(&txbody, &txwitness, None);
    let out = transaction.to_bytes();
    let tx = hex::encode(out);

    // Conserve TxWitness
    let hex_txwitness = hex::encode(txwitness.to_bytes());

    // Conserve aux data
    let aux_out = aux_data.to_bytes();
    let hex_aux = hex::encode(aux_out);

    // Conserve txBody in json file
    let txbody_out = txbody.to_bytes();
    let hex_body = hex::encode(txbody_out);

    let mut it = "";
    if internal {
        it = "it";
    }

    let jout = BuildOutput {
        tx_witness: hex_txwitness,
        metadata: hex_aux,
        tx_body: hex_body,
        tx_unsigned: tx,
        used_utxos,
        royalties,
        internal_transfer: it.to_string(),
    };

    //trasaction to json file
    //serde_json::to_writer(&File::create("stored.artifct")?,&jout)?;
    //serde_json::to_writer(&io::stdout(),&jout)?;

    //Ok(serde_json::to_string(&jout)?)
    Ok(jout)
}

pub fn make_script_outputs(
    tx: &mut ctxb::TransactionBuilder,
    datum: &ccrypto::DataHash,
    script_outputs: &Vec<ScriptOutput>,
    sc_addr: String,
) -> Option<(cutils::Value, TransactionUnspentOutput)> {
    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //
    //Add Script Outputs
    //  Add the NFT from the users wallet to the smart contract
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    if script_outputs.len() == 1 {
        if let Some(o) = script_outputs.iter().next() {
            let r_address = &caddr::Address::from_bech32(&sc_addr).unwrap();
            debug!("Address NetworkId: {:?}", r_address.network_id());
            let mut value = cutils::Value::new(&cutils::to_bignum(0u64));
            let mut lovelaces: u64 = 0;
            let mut multiasset = clib::MultiAsset::new();
            for v in &o.value {
                if v.currencySymbol.is_empty() || v.currencySymbol == "lovelace" {
                    for a in &v.assets {
                        lovelaces += a.amount;
                    }
                } else {
                    let cs: clib::PolicyID =
                        clib::PolicyID::from_bytes(hex::decode(&v.currencySymbol).unwrap())
                            .unwrap();
                    let mut assets = clib::Assets::new();
                    for a in &v.assets {
                        let tn: clib::AssetName =
                            clib::AssetName::new(hex::decode(&a.tokenName).unwrap()).unwrap();
                        assets.insert(&tn, &cutils::to_bignum(a.amount));
                        //info!("{:?}.{:?}",cs,assets);
                        multiasset.insert(&cs, &assets);
                    }
                }
            }
            value.set_coin(&cutils::to_bignum(lovelaces));
            value.set_multiasset(&multiasset);
            let min_ada_utxo = txbuilders::calc_min_ada_for_utxo(&value, Some(datum.clone()));
            value.set_coin(&min_ada_utxo);
            //info!("For Output: {:?} added {:?} lovelaces and {:?} tokens",o.address,lovelaces,multiasset);

            let mut txout = clib::TransactionOutput::new(r_address, &value);
            txout.set_data_hash(datum);
            let txin = clib::TransactionInput::new(
                &ccrypto::TransactionHash::from_bytes(hex::decode(o.txhash.clone()).unwrap())
                    .unwrap(),
                o.txinput,
            );
            tx.add_output(&txout).unwrap();

            return Some((value, TransactionUnspentOutput::new(&txin, &txout)));
        }
    }
    None
}

pub fn _make_wallet_outputs(
    tx: &mut ctxb::TransactionBuilder,
    outputs: &Vec<TxOutput>,
    mut manual_fee: bool,
) -> u32 {
    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //
    //Add wallet outputs
    //  If existing add outputs to another wallet or changes to the issuers wallet
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    for o in outputs {
        let r_address = &caddr::Address::from_bech32(&o.address).unwrap();
        let mut value = cutils::Value::new(&cutils::to_bignum(0u64));
        let mut lovelaces: u64 = 0;
        let mut multiasset = clib::MultiAsset::new();
        for v in &o.value {
            if v.currencySymbol.is_empty() || v.currencySymbol == "lovelace" {
                for a in &v.assets {
                    if manual_fee && a.amount > 4000000 {
                        lovelaces = lovelaces + a.amount - 2000000;
                        manual_fee = false;
                    } else {
                        lovelaces += a.amount
                    }
                }
            } else {
                let cs: clib::PolicyID =
                    clib::PolicyID::from_bytes(hex::decode(&v.currencySymbol).unwrap()).unwrap();
                let mut assets = clib::Assets::new();
                for a in &v.assets {
                    let tn: clib::AssetName =
                        clib::AssetName::new(hex::decode(&a.tokenName).unwrap()).unwrap();
                    assets.insert(&tn, &cutils::to_bignum(a.amount));
                    //info!("{:?}.{:?}",cs,assets);
                    multiasset.insert(&cs, &assets);
                }
            }
        }
        value.set_coin(&cutils::to_bignum(lovelaces));
        value.set_multiasset(&multiasset);
        //info!("For Output: {:?} added {:?} lovelaces and {:?} tokens",o.address,lovelaces,multiasset);
        let txout = clib::TransactionOutput::new(r_address, &value);
        tx.add_output(&txout).unwrap();
    }
    //info!();
    0u32
}

pub fn make_script_outputs_txb(
    txos: &mut clib::TransactionOutputs,
    receiver: String,
    datum: &ccrypto::DataHash,
    script_outputs: &Vec<ScriptOutput>,
    set_datum: bool,
) -> Option<cutils::TransactionUnspentOutput> {
    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //
    //Add Script Outputs for Transaction Body
    //  Add the NFT from the smart contract
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    //let mut txuos = TransactionUnspentOutputs::new();
    let mut txuo: Option<cutils::TransactionUnspentOutput> = None;

    //let mut i = 0;
    if script_outputs.len() == 1 {
        for o in script_outputs {
            let r_address = &caddr::Address::from_bech32(&receiver).unwrap();
            let mut value = cutils::Value::new(&cutils::to_bignum(0u64));
            let mut lovelaces: u64 = 0;
            let mut multiasset = clib::MultiAsset::new();
            for v in &o.value {
                if v.currencySymbol.is_empty() || v.currencySymbol == "lovelace" {
                    for a in &v.assets {
                        lovelaces += a.amount;
                    }
                } else {
                    let cs: clib::PolicyID =
                        clib::PolicyID::from_bytes(hex::decode(&v.currencySymbol).unwrap())
                            .unwrap();
                    let mut assets = clib::Assets::new();
                    for a in &v.assets {
                        let tn: clib::AssetName =
                            clib::AssetName::new(hex::decode(&a.tokenName).unwrap()).unwrap();
                        assets.insert(&tn, &cutils::to_bignum(a.amount));
                        //info!("{:?}.{:?}",cs,assets);
                        multiasset.insert(&cs, &assets);
                    }
                }
            }
            value.set_coin(&cutils::to_bignum(lovelaces));
            value.set_multiasset(&multiasset);
            let min_ada_utxo = txbuilders::calc_min_ada_for_utxo(&value, Some(datum.clone()));
            value.set_coin(&min_ada_utxo);
            //info!("For Output: {:?} added {:?} lovelaces and {:?} tokens",o.address,lovelaces,multiasset);
            let mut txout = clib::TransactionOutput::new(r_address, &value);
            let txin = clib::TransactionInput::new(
                &ccrypto::TransactionHash::from_bytes(hex::decode(o.txhash.clone()).unwrap())
                    .unwrap(),
                o.txinput,
            );

            if set_datum {
                txout.set_data_hash(datum)
            };
            txos.add(&txout);

            txuo = Some(cutils::TransactionUnspentOutput::new(&txin, &txout));
            //txuos.add(&cutils::TransactionUnspentOutput::new(&txin,&txout));
            //debug!("Script UnspentOutput: {:?}",txuo);
            //i+=1;
        }
    }
    txuo
}

pub fn sum_output_values(txouts: &clib::TransactionOutputs) -> cutils::Value {
    let mut acc = cutils::Value::new(&cutils::to_bignum(64));
    for i in 0..txouts.len() {
        acc = acc.checked_add(&txouts.get(i).amount()).unwrap();
    }

    acc
}

pub fn splitt_ada_off(
    value: cutils::Value,
    addr: caddr::Address,
    split_txos: &mut clib::TransactionOutputs,
) {
    let min_utxo_ada = txbuilders::calc_min_ada_for_utxo(&value, None);
    let coins = value.coin();
    match coins.compare(
        &min_utxo_ada
            .checked_add(&cutils::to_bignum(MIN_ADA * 5))
            .unwrap(),
    ) {
        1 => {
            let coins = &coins.checked_sub(&min_utxo_ada).unwrap();
            let coin_value = cutils::Value::new(coins);
            let coin_txo = clib::TransactionOutput::new(&addr, &coin_value);

            let mut token_value = cutils::Value::new(&min_utxo_ada);
            token_value.set_multiasset(&value.multiasset().unwrap());
            split_txos.add(&clib::TransactionOutput::new(&addr, &token_value));
            split_txos.add(&coin_txo);
        }
        _ => {
            split_txos.add(&clib::TransactionOutput::new(&addr, &value));
        }
    }
}

pub fn split_output_txo(txo: clib::TransactionOutput, split_txos: &mut clib::TransactionOutputs) {
    let input_txo = txo.clone();
    const T_TH: usize = 40;
    const C_TH: u64 = 40000000;
    const B_TH: usize = 2500;
    const B_MAX: usize = 5000;
    match txo.data_hash() {
        Some(_) => {
            // Split not allowed is smart contract utxo
            split_txos.add(&txo);
        }
        None => {
            let value = txo.amount();
            let addr = txo.address();
            match value.multiasset() {
                None => {
                    // Ada only leav it as it is
                    split_txos.add(&txo);
                }
                Some(multi) => {
                    debug!("Multi Asset in Splitter");
                    if multi.len() == 0 {
                        // Ada only leave it as it is
                        debug!("Multi Len <= 0 , {:?}", multi);
                        split_txos.add(&txo);
                    } else {
                        // Check if a split makes sense

                        //Limit max Val size
                        let txo_size = input_txo.to_bytes().len();
                        debug!("Txo Size: {:?}", txo_size);
                        debug!("ValSize:  {:?}", value.to_bytes().len());
                        if txo_size < B_MAX {
                            // Value is small leave it as it is
                            if value.coin().compare(&cutils::to_bignum(C_TH)) >= 0 {
                                splitt_ada_off(value, addr, split_txos);
                            } else {
                                split_txos.add(&txo);
                            }
                        } else {
                            match multi.keys().len() {
                                //Split for one Policy ID in Value
                                1 => {
                                    debug!("One policy ID");
                                    let cs = &multi.keys().get(0);
                                    let assets = multi.get(cs).unwrap();

                                    match assets.len() {
                                        1..=T_TH => {
                                            // Is okay all are form one policy so the utxo has a good size
                                            //Check if we can split away Ada:
                                            splitt_ada_off(value, addr, split_txos);
                                        }

                                        _ => {
                                            let mut new_asset = clib::Assets::new();
                                            let mut rest_asset = clib::Assets::new();
                                            let keys = assets.keys();
                                            for a in 0..assets.len() {
                                                if a <= T_TH {
                                                    let name = &keys.get(a);
                                                    let amt = &assets.get(name).unwrap();
                                                    new_asset.insert(name, amt);
                                                } else {
                                                    let name = &keys.get(a);
                                                    let amt = &assets.get(name).unwrap();
                                                    rest_asset.insert(name, amt);
                                                }
                                            }
                                            // First Value for TxOut
                                            let mut new_value =
                                                cutils::Value::new(&cutils::to_bignum(2000000));
                                            let mut new_multi = clib::MultiAsset::new();
                                            new_multi.insert(cs, &new_asset);
                                            new_value.set_multiasset(&new_multi);
                                            let min_utxo_new_val =
                                                txbuilders::calc_min_ada_for_utxo(&new_value, None);
                                            new_value.set_coin(&min_utxo_new_val);

                                            // Value for Recursive call
                                            let coin_dif = value
                                                .coin()
                                                .checked_sub(&min_utxo_new_val)
                                                .unwrap();
                                            let mut rest_value = cutils::Value::new(&coin_dif);
                                            if rest_asset.len() > 0 {
                                                let mut rest_multi = clib::MultiAsset::new();
                                                rest_multi.insert(cs, &rest_asset);
                                                rest_value.set_multiasset(&rest_multi);
                                            }
                                            let min_utxo = txbuilders::calc_min_ada_for_utxo(
                                                &rest_value,
                                                None,
                                            );
                                            let rest_txo =
                                                clib::TransactionOutput::new(&addr, &rest_value);

                                            if coin_dif.compare(&min_utxo) == -1 {
                                                // Not enough Ada to cover min utxo on a split leave the utxo as it is
                                                split_txos.add(&txo);
                                            } else {
                                                let nex_txo = &clib::TransactionOutput::new(
                                                    &addr, &new_value,
                                                );
                                                split_txos.add(nex_txo);
                                                debug!("\n----New TXO:-------\n {:?}\n", nex_txo);
                                                debug!("\nRest TXO: {:?}", rest_txo);
                                                debug!("Recursion");
                                                split_output_txo(rest_txo, split_txos);
                                            }
                                        }
                                    }
                                }

                                // more than one policy Id optimize on utxo byte size
                                _ => {
                                    debug!("Many policy ID");
                                    let mut i: usize = 0;
                                    let mut j: usize = 0;
                                    let work_multi = multi;

                                    let mut worker_val =
                                        cutils::Value::new(&cutils::to_bignum(2000000u64));
                                    let mut new_txo =
                                        clib::TransactionOutput::new(&addr, &worker_val);

                                    loop {
                                        match work_multi.get(&work_multi.keys().get(i)) {
                                            None => {
                                                break;
                                            }
                                            Some(assets) => {
                                                if assets.len() == 0 || j >= assets.len() {
                                                    i += 1;
                                                    j = 0;
                                                    continue;
                                                } else {
                                                    let name = assets.keys().get(j);
                                                    let amt = &assets.get(&name).unwrap();
                                                    let mut new_token = clib::Assets::new();
                                                    new_token.insert(&name, amt);
                                                    let mut new_multi = clib::MultiAsset::new();
                                                    new_multi.insert(
                                                        &work_multi.keys().get(i),
                                                        &new_token,
                                                    );
                                                    let mut temp_val = cutils::Value::new(
                                                        &cutils::to_bignum(0u64),
                                                    );
                                                    temp_val.set_multiasset(&new_multi);
                                                    worker_val =
                                                        worker_val.checked_add(&temp_val).unwrap();
                                                    j += 1;
                                                }
                                                worker_val.set_coin(
                                                    &txbuilders::calc_min_ada_for_utxo(
                                                        &worker_val,
                                                        None,
                                                    ),
                                                );
                                                let dtxo = clib::TransactionOutput::new(
                                                    &addr,
                                                    &worker_val,
                                                );
                                                if dtxo.to_bytes().len() >= B_TH {
                                                    new_txo = dtxo;
                                                    debug!("New TXO: {:?}", new_txo);
                                                    break;
                                                }
                                            }
                                        }
                                        //if name.to_bytes() == hex::decode("5436").unwrap() || name.to_bytes() == hex::decode("5435").unwrap() {
                                        //  info!("Name! {:?}\n",name.to_bytes());
                                        //}
                                    }
                                    match value.coin().checked_sub(
                                        &new_txo
                                            .amount()
                                            .coin()
                                            .checked_add(&cutils::to_bignum(MIN_ADA))
                                            .unwrap(),
                                    ) {
                                        Ok(_) => {
                                            debug!("here I am");
                                            let rest_val =
                                                value.checked_sub(&new_txo.amount()).unwrap();
                                            match rest_val.multiasset() {
                                                None => {
                                                    let added_val = new_txo.amount();
                                                    //added_val.set_coin(&rest_val.coin().checked_add(&new_txo.amount().coin()).unwrap());
                                                    new_txo = clib::TransactionOutput::new(
                                                        &addr, &added_val,
                                                    );
                                                }

                                                Some(multi) => {
                                                    debug!(
                                                        "Bla: {:?}",
                                                        multi
                                                            .get(&multi.keys().get(0))
                                                            .unwrap()
                                                            .len()
                                                    );
                                                    if multi.len() == 0
                                                        || multi
                                                            .get(&multi.keys().get(0))
                                                            .unwrap()
                                                            .len()
                                                            == 0
                                                    {
                                                        let mut added_val = new_txo.amount();
                                                        added_val.set_coin(
                                                            &rest_val
                                                                .coin()
                                                                .checked_add(
                                                                    &new_txo.amount().coin(),
                                                                )
                                                                .unwrap(),
                                                        );
                                                        new_txo = clib::TransactionOutput::new(
                                                            &addr, &added_val,
                                                        );
                                                    }
                                                }
                                            }
                                            split_txos.add(&new_txo);
                                            debug!("Recursion");
                                            debug!("REST VAL: {:?}", rest_val);
                                            split_output_txo(
                                                clib::TransactionOutput::new(&addr, &rest_val),
                                                split_txos,
                                            );
                                        }

                                        Err(_) => {
                                            split_txos.add(&new_txo);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn splitt_coin_multi(
    txins: &TransactionUnspentOutputs,
) -> (TransactionUnspentOutputs, TransactionUnspentOutputs) {
    let mut ada_only = TransactionUnspentOutputs::new();
    let mut multi = TransactionUnspentOutputs::new();

    for tx in txins.clone() {
        match tx.output().amount().multiasset() {
            Some(x) => {
                if x.len() > 0 {
                    multi.add(&tx)
                } else {
                    ada_only.add(&tx)
                }
            }
            None => ada_only.add(&tx),
        }
    }
    ada_only.sort_by_coin();
    multi.sort_by_multi_amount();

    (ada_only, multi)
}

pub fn find_suitable_coins(
    nv: &mut cutils::Value,
    inputs: &mut TransactionUnspentOutputs,
    overhead: u64,
) -> (Option<TransactionUnspentOutputs>, u64) {
    //Fee need to be considered in needed value

    let coins = cutils::from_bignum(&nv.coin());
    let max_coins = coins + (coins / 100 * overhead); // Coins + Overhead in %

    let mut acc = 0u64;
    let mut selection = TransactionUnspentOutputs::new();
    let mut multi_storage = TransactionUnspentOutputs::new();
    let mut coin_storage = TransactionUnspentOutputs::new();

    //debug!("\n\nTXINS in find suitabe coins: {:?}\n\n", inputs);

    //debug!(
    //    "\nFIND COINS: max_coins {:?}, coins: {:?}\n",
    //    max_coins, coins
    //);
    'outer: for tx in inputs.clone() {
        debug!(
            "\n-------TXIn : {:?}#{:?}",
            tx.input().transaction_id().to_hex(),
            tx.input().index()
        );

        let lc = cutils::from_bignum(&tx.output().amount().coin());
        //debug!(
        //    "\n---LC: {:?}----TXIn : {:?}{:?}",
        //    lc,
        //    tx.input().transaction_id(),
        //    tx.input().index()
        //);
        if lc > coins {
            match tx.output().amount().multiasset() {
                Some(multi) => match multi.len() {
                    0 => {
                        //debug!("Multiasset of Len 0: Found Coins: {:?}", lc);
                        if lc < max_coins {
                            //debug!("No Multiasset: Found Coins: {:?}", lc);
                            selection.add(&tx);
                            debug!("Selection: {:?}", selection);
                            return (Some(selection), lc);
                        } else {
                            coin_storage.add(&tx);
                        }
                    }

                    1..=21 => {
                        //debug!("Multiasses with less than 21 NFTs, Found Coins: {:?}", lc);
                        selection.add(&tx);
                        //debug!("Selection: {:?}", selection);
                        return (Some(selection), lc);
                    }
                    _ => {
                        //debug!("More than 21 NFTs, Found Coins: {:?}", lc);
                        //debug!("Trying find better option, store this one");
                        multi_storage.add(&tx);
                    }
                },

                None => {
                    if lc < max_coins {
                        //debug!("No Multiasset: Found Coins: {:?}", lc);
                        selection.add(&tx);
                        debug!("Selection: {:?}", selection);
                        return (Some(selection), lc);
                    } else {
                        coin_storage.add(&tx);
                    }
                }
            }
        }
        if lc <= coins {
            if !coin_storage.is_empty() {
                debug!("Took from coinstorage");
                coin_storage.sort_by_coin();
                let tx = coin_storage.get(0);
                selection.add(&tx);
                acc = cutils::from_bignum(&tx.output().amount().coin());
                return (Some(selection), acc);
            }
            debug!("Break out all coins to small");
            break 'outer;
        }
    }
    if !coin_storage.is_empty() {
        coin_storage.sort_by_coin();
        //debug!("Took from coinstorage: {:?}", coin_storage);

        let tx = coin_storage.get(0);
        selection.add(&tx);
        acc = cutils::from_bignum(&tx.output().amount().coin());
        return (Some(selection), acc);
    } else {
        for tx in inputs {
            //debug!(
            //    "\n-------TXIn in Acc : {:?}{:?}",
            //    tx.input().transaction_id(),
            //    tx.input().index()
            //);
            let lc = cutils::from_bignum(&tx.output().amount().coin());
            acc += lc;
            //debug!("Acc {:?}, LC: {:?}", acc, lc);
            selection.add(&tx);
            if acc > coins + MIN_ADA {
                //debug!("Return in Accumulator {:?}", acc);
                return (Some(selection), acc);
            }
        }
    }

    //debug!("SUITABLE COINS: {:?}", acc);

    if selection.is_empty() {
        debug!("Selection length = 0");
        (None, 0)
    } else {
        if !multi_storage.is_empty() {
            let mut selection = TransactionUnspentOutputs::new();
            multi_storage.sort_by_multi_amount();
            let tx = multi_storage.get(0);
            selection.add(&tx);
            acc = cutils::from_bignum(&tx.output().amount().coin());
        }
        (Some(selection), acc)
    }
}

pub fn find_asset_utxo(
    txuos: TransactionUnspentOutputs,
    nft_cs: ccrypto::ScriptHash,
    nft_tn: clib::AssetName,
) -> Option<usize> {
    if !txuos.is_empty() {
        debug!("len > 1:  {:?}", txuos);
        for i in 0..txuos.len() {
            let unspent_output = txuos.get(i);
            let value = unspent_output.output().amount();
            match value.multiasset() {
                Some(multi) => match multi.get(&nft_cs) {
                    Some(assets) => {
                        for p in 0..assets.len() {
                            if assets.keys().get(p) == nft_tn {
                                debug!("Found Utxo with Token!");
                                return Some(i);
                            }
                        }
                    }
                    None => {
                        debug!("No Assets found for Policy ID:{:?}  found", nft_cs);
                        debug!("Multi: {:?}\n", multi);
                    }
                },
                None => {
                    debug!("No Policy Id found");
                }
            }
        }
    }
    None
}

pub fn find_asset_utxos_in_txuos(
    txuos: &TransactionUnspentOutputs,
    listing_tokens: &Vec<(ccrypto::ScriptHash, clib::AssetName, cutils::BigNum)>,
) -> Vec<usize> {
    let mut ret = Vec::<usize>::new();
    if !txuos.is_empty() {
        debug!("len > 1:  {:?}", txuos);
        for token in listing_tokens {
            let cs = &token.0;
            let tn = &token.1;

            for i in 0..txuos.len() {
                let unspent_output = txuos.get(i);
                let value = unspent_output.output().amount();
                match value.multiasset() {
                    Some(multi) => match multi.get(cs) {
                        Some(assets) => {
                            for p in 0..assets.len() {
                                if assets.keys().get(p) == *tn {
                                    debug!("Found Utxo with Token!");
                                    ret.push(i);
                                }
                            }
                        }
                        None => {
                            debug!("No Assets found for Policy ID:{:?}  found", cs);
                            debug!("Multi: {:?}\n", multi);
                        }
                    },
                    None => {
                        debug!("No Policy Id found");
                    }
                }
            }
        }
    }
    ret
}

pub fn find_collateral_by_txhash_txix(
    elem: &TransactionUnspentOutput,
    txuos: &TransactionUnspentOutputs,
) -> Option<usize> {
    let col_max = cutils::to_bignum(20000000u64);
    let elem_hash = elem.input().transaction_id();
    let elem_index = elem.input().index();
    debug!("Elemhash: {:?}, ElemIndex: {:?}", elem_hash, elem_index);
    for i in 0..txuos.len() {
        let txi = txuos.get(i).input();
        debug!("Input: {:?}", txi);
        if txi.transaction_id().to_bytes() == elem_hash.to_bytes() && txi.index() == elem_index {
            debug!("Found collateral utxo");
            if txuos.get(i).output().amount().coin().compare(&col_max) <= 0 {
                return Some(i);
            } else {
                return None;
            }
        }
    }
    None
}

pub fn find_utxos_by_address(
    addr: caddr::Address,
    txuos: &TransactionUnspentOutputs,
) -> (TransactionUnspentOutputs, TransactionUnspentOutputs) {
    let mut addr_utxos = TransactionUnspentOutputs::new();
    let mut other_utxos = TransactionUnspentOutputs::new();

    for tx in txuos.clone() {
        if tx.output().address().to_bytes() == addr.to_bytes() {
            addr_utxos.add(&tx);
        } else {
            other_utxos.add(&tx);
        }
    }

    addr_utxos.sort_by_multi_amount();
    other_utxos.sort_by_coin();

    (addr_utxos, other_utxos)
}

pub fn input_selection(
    token_utxos: Option<&TransactionUnspentOutputs>,
    needed_value: &mut cutils::Value,
    txins: &TransactionUnspentOutputs,
    collateral: Option<cutils::TransactionUnspentOutput>,
) -> (clib::TransactionInputs, TransactionUnspentOutputs) {
    debug!("\n\nMULTIASSETS: {:?}\n\n", txins);

    let (mut purecoinassets, mut multiassets) = splitt_coin_multi(txins);

    let mut nv = needed_value.clone();
    let mut selection = TransactionUnspentOutputs::new();
    let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));
    let mut txins = clib::TransactionInputs::new();

    let overhead = 50u64;

    if let Some(token_utxos) = token_utxos {
        for i in 0..token_utxos.len() {
            selection.add(&token_utxos.get(i));
            acc = acc
                .checked_add(&token_utxos.get(i).output().amount())
                .unwrap();
            nv = nv
                .checked_add(&token_utxos.get(i).output().amount())
                .unwrap();
            debug!("\n\nAdded Script Utxo to Acc Value : \n {:?}\n", acc);
            // Delete script input from multi assets
            if let Some(i) = multiassets.find_utxo_index(&token_utxos.get(i)) {
                let tutxo = multiassets.swap_remove(i);
                debug!(
                    "Deleted token utxo from multiasset inputs: \n {:?}\n",
                    tutxo
                );
            }
        }
    }
    //let mut missing_value = needed_value.clone(); needed to search for multiassets

    if let Some(cutxo) = collateral {
        debug!("Col: {:?}", cutxo);
        let c_index = find_collateral_by_txhash_txix(&cutxo, &purecoinassets);
        debug!(
            "Some collateral to check for deletion found, Index: {:?}",
            c_index
        );
        if let Some(index) = c_index {
            let col = purecoinassets.swap_remove(index);
            debug!("Deleted collateral from inputs: {:?}\n", col);
            // Double check
            if find_collateral_by_txhash_txix(&cutxo, &purecoinassets).is_some() {
                panic!("PANIC COLLATERAL COULDN'T BE EXCLUDED FROM SELECTION SET");
            }
        }
    }

    multiassets.sort_by_coin(); //.sort_by_multi_amount();
    purecoinassets.sort_by_coin();

    debug!("\n\nMULTIASSETS: {:?}\n\n", multiassets);
    debug!("\n\npurecoinassets: {:?}\n\n", purecoinassets);

    let utxo_count = multiassets.len() + purecoinassets.len();
    let mut max_run = 0;
    debug!("\n\nNV: {:?}", nv);
    debug!("\n\nNV: {:?}", acc);
    debug!(
        "\nbefore while! Utxo Count: {:?}, {:?} \n",
        utxo_count,
        (nv.coin().compare(&acc.coin()) > 0)
    );
    while nv.coin().compare(&acc.coin()) > 0 && max_run < utxo_count {
        nv = nv.checked_sub(&acc).unwrap();

        if purecoinassets.is_empty() {
            // Find the tokens we want in the multis
            debug!("\nWe look for multiassets!\n");
            let ret = find_suitable_coins(&mut nv, &mut multiassets, overhead);
            match ret.0 {
                Some(utxos) => {
                    for u in utxos {
                        selection.add(&u);
                    }
                    acc.set_coin(&acc.coin().checked_add(&cutils::to_bignum(ret.1)).unwrap());
                }
                None => {
                    //ToDo: Do not panic -> Error
                    panic!("ERROR: Not enough input utxos available to balance the transaction");
                }
            }
            let _ = multiassets.pop();
        } else {
            // Fine enough Ada to pay the transaction
            let ret = find_suitable_coins(&mut nv, &mut purecoinassets, overhead);
            debug!("Return coinassets: {:?}", ret);
            match ret.0 {
                Some(utxos) => {
                    for u in utxos {
                        selection.add(&u);
                    }
                    acc.set_coin(&acc.coin().checked_add(&cutils::to_bignum(ret.1)).unwrap());
                    debug!("\nSelection in coinassets: {:?}", selection);
                    debug!("\nAcc in coinassets: {:?}", acc);
                }
                None => {
                    panic!("ERROR: Not enough input utxos available to balance the transaction")
                }
            }
            let _ = purecoinassets.pop();
        }
        max_run += 1;
    }
    for txuo in selection.clone() {
        txins.add(&txuo.input());
    }
    debug!("\n\nSelection: {:?}\n\n", selection);
    (txins, selection)
}

#[allow(clippy::too_many_arguments)]
pub fn balance_tx(
    input_txuos: &mut TransactionUnspentOutputs,
    // script_outputs : Option<&TransactionUnspentOutput>,
    _tokens: &Tokens, //&Vec<(ccrypto::ScriptHash, clib::AssetName, BigNum)>,
    txos: &mut clib::TransactionOutputs,
    already_paid: Option<&cutils::Value>,
    fee: &cutils::BigNum,
    fee_paied: &mut bool,
    first_run: &mut bool,
    txos_paied: &mut bool,
    tbb_values: &mut cutils::Value,
    senders_addr: &caddr::Address,
    change_address: &caddr::Address,
    acc_change: &mut cutils::Value,
    sc_addr: Option<caddr::Address>,
    dummyrun: &bool,
) -> Result<clib::TransactionOutputs, MurinError> {
    if *first_run {
        if txos.len() > 0 {
            for i in 0..txos.len() {
                *tbb_values = tbb_values.checked_add(&txos.get(i).amount()).unwrap();
                debug!(
                    "\nAdding value from existing outputs to to_be_paid: {:?}",
                    tbb_values.coin()
                );
            }
        }
        let fee_val = cutils::Value::new(fee);
        *tbb_values = tbb_values.checked_add(&fee_val).unwrap();

        // If something is balanced out of this function here an offset can be set
        if let Some(is_paid) = already_paid {
            *tbb_values = tbb_values.checked_sub(is_paid).unwrap();
        }

        debug!("\nAdded fee to to_be_paid: {:?}", tbb_values.coin());
        // When everything is done set first_run to false
        *first_run = false;
    }
    debug!("tbb value after first run: {:?}", tbb_values.coin());
    let option_next_input = input_txuos.pop();
    debug!("next input: {:?}", option_next_input);
    // Recursivly balance the transaction depending on what inputs we have
    match option_next_input {
        Some(unspent_output) => {
            // cutils::TransactionUnspentOutput {input, output}
            let value = unspent_output.output().amount();
            let input_address_pkey = get_stake_address(&unspent_output.output().address());
            let senders_addr_pkey = get_stake_address(senders_addr);
            let enterprise = if let Some(ent) =
                EnterpriseAddress::from_address(&unspent_output.output().address())
            {
                ent.payment_cred().kind()
            } else {
                StakeCredKind::Script
            };
            let base = BaseAddress::from_address(senders_addr);
            debug!(
                "\nInputs Address: {:?}",
                hex::encode(input_address_pkey.to_bytes())
            );
            debug!(
                "Senders Key: {:?}\n",
                hex::encode(senders_addr_pkey.to_bytes())
            );
            let matching_addresses = (input_address_pkey == senders_addr_pkey)
                || (base.is_some() && enterprise == StakeCredKind::Key);
            if matching_addresses {
                *acc_change = acc_change.checked_add(&value).unwrap();
            } else {
                // TODO: Make SC ADDR Variable to be able to use different Smart Contracts
                if let Some(sc) = sc_addr.clone() {
                    if input_address_pkey == get_stake_address(&sc) {
                        *acc_change = acc_change.checked_add(&value)?;
                    }
                }
            }
            // call this function recursivley
            balance_tx(
                input_txuos,
                _tokens,
                txos,
                already_paid,
                fee,
                fee_paied,
                first_run,
                txos_paied,
                tbb_values,
                senders_addr,
                change_address,
                acc_change,
                sc_addr,
                dummyrun,
            )
        }
        None => {
            // If we had many small utxos they got accumulated, we now substract it from the accumulation
            if acc_change.coin().compare(&tbb_values.coin()) >= 0 {
                debug!("\nPay: {:?} with value: {:?}", tbb_values, acc_change);
                *acc_change = acc_change.clamped_sub(tbb_values);
                *fee_paied = true;
                *txos_paied = true;
                debug!("------------------------------------------------------------------\n\n");
                debug!("Acc after clamped sub: \n{:?}", acc_change);
                debug!("------------------------------------------------------------------\n\n");
            }
            let min_utxo = txbuilders::calc_min_ada_for_utxo(acc_change, None);

            let min_ada_value = cutils::Value::new(&min_utxo);
            if *txos_paied && *fee_paied && acc_change.coin().compare(&min_ada_value.coin()) >= 0 {
                debug!("\nAdded accumulated output: {:?}", acc_change.coin());
                let acc_txo = clib::TransactionOutput::new(change_address, acc_change);
                let mut out_txos = clib::TransactionOutputs::new();
                debug!(
                    "\n\nBefore Splitter: ACC: {:?}\n\n TXO: {:?}",
                    acc_txo, out_txos
                );
                split_output_txo(acc_txo, &mut out_txos);

                for i in 0..out_txos.len() {
                    debug!("TXOS in out_txos {:?}", out_txos.get(i));
                    txos.add(&out_txos.get(i));
                }
                *acc_change = acc_change.checked_sub(acc_change).unwrap();
            } else if (acc_change.coin().compare(&min_utxo) == -1
                && acc_change.coin().compare(&cutils::to_bignum(0u64)) != 0)
                && !*dummyrun
            {
                panic!("\nERROR: Transaction does balance but last output is below min Ada value: {:?} overhead",acc_change.coin());
            }
            if (!*txos_paied
                || !*fee_paied
                || acc_change.coin().compare(&cutils::to_bignum(0u64)) != 0
                || acc_change.multiasset().is_some())
                && !*dummyrun
            {
                let mut overhead = &mut cutils::Value::new(&cutils::to_bignum(0u64));
                if !*txos_paied {
                    overhead = tbb_values;
                }
                if !*fee_paied {
                    overhead.set_coin(&overhead.coin().checked_add(fee).unwrap());
                }
                panic!("\nERROR: Transaction does not balance: {overhead:?} overhead, fee paied: {fee_paied:?}, outputs paied: {txos_paied:?}");
            }
            debug!("Accumulated Change is Zero?: {:?}", acc_change);
            Ok(txos.clone())
        }
    }
}

pub fn make_inputs_txb(
    inputs: &Vec<TxInput>,
) -> (clib::TransactionInputs, TransactionUnspentOutputs) {
    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //
    //Add wallet inputs for the transaction
    //  Add the inputs for this transaction
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    let mut txuos = TransactionUnspentOutputs::new(); // Available in 9.2.0 beta not in 9.1.2
    let mut txins = clib::TransactionInputs::new();
    for i in inputs {
        let txuo_in = &clib::TransactionInput::new(
            &ccrypto::TransactionHash::from_bytes(hex::decode(i.txhash.clone()).unwrap()).unwrap(),
            i.txinput,
        );

        let mut value = cutils::Value::new(&cutils::to_bignum(0u64));
        let mut lovelaces: u64 = 0;
        let mut multiasset = clib::MultiAsset::new();
        for v in &i.value {
            if v.currencySymbol.is_empty() || v.currencySymbol == "lovelace" {
                for a in &v.assets {
                    lovelaces += a.amount;
                }
            } else {
                let cs: clib::PolicyID =
                    clib::PolicyID::from_bytes(hex::decode(&v.currencySymbol).unwrap()).unwrap();
                let mut assets = clib::Assets::new();
                for a in &v.assets {
                    let tn: clib::AssetName =
                        clib::AssetName::new(hex::decode(&a.tokenName).unwrap()).unwrap();
                    assets.insert(&tn, &cutils::to_bignum(a.amount));
                    //info!("{:?}.{:?}",cs,assets);
                    multiasset.insert(&cs, &assets);
                }
            }
        }
        value.set_coin(&cutils::to_bignum(lovelaces));
        value.set_multiasset(&multiasset);
        let addr = &caddr::Address::from_bech32(&i.address).unwrap();
        let txuo_out = clib::TransactionOutput::new(addr, &value);
        let txuo: cutils::TransactionUnspentOutput =
            cutils::TransactionUnspentOutput::new(txuo_in, &txuo_out);
        //info!("TXOU: {:?}",txuo);
        //info!();

        txuos.add(&txuo);
        txins.add(txuo_in);
    }

    (txins.clone(), txuos.clone())
}

pub fn make_datum_mp(
    selling_price: &String,
    trade_owner: &str,
    royalties_rate: &String,
    policy_id: &String,
    token_name: &String,
    royalties_pkh: &ccrypto::Ed25519KeyHash,
    sc_version: &String,
) -> (ccrypto::DataHash, plutus::PlutusList, Vec<Vec<u8>>) {
    let trade_address = &caddr::Address::from_bech32(trade_owner).unwrap();
    let wallet_address = &caddr::BaseAddress::from_address(trade_address).unwrap();
    let pub_key_hash = hex::encode(
        wallet_address
            .payment_cred()
            .to_keyhash()
            .unwrap()
            .to_bytes(),
    );
    let stake_key_hash = hex::encode(
        wallet_address
            .stake_cred()
            .to_keyhash()
            .unwrap()
            .to_bech32("addr_stake")
            .unwrap()
            .as_bytes(),
    );
    debug!(
        "PubKeyHash: {:?}, StakeKeyHash: {:?}\n",
        pub_key_hash, stake_key_hash
    );
    debug!("Original baseAddress: {:?}", wallet_address);

    let roy_pkey_hash = royalties_pkh.to_bytes();

    let nft_policy_id = clib::PolicyID::from_bytes(hex::decode(policy_id).unwrap()).unwrap();
    let nft_token_name = hex::decode(token_name).unwrap();

    let mut fields_inner = plutus::PlutusList::new();
    fields_inner.add(&plutus::PlutusData::new_integer(
        &cutils::BigInt::from_str(selling_price).unwrap(),
    )); // Selling Price
    fields_inner.add(&plutus::PlutusData::new_bytes(
        hex::decode(pub_key_hash.clone()).unwrap(),
    )); // Sellers PubKeyHash
    fields_inner.add(&plutus::PlutusData::new_integer(
        &cutils::BigInt::from_str(royalties_rate).unwrap(),
    )); // royalties rate in promille
    fields_inner.add(&plutus::PlutusData::new_bytes(roy_pkey_hash.clone())); // Royalties PubKeyHash
    fields_inner.add(&plutus::PlutusData::new_bytes(
        hex::decode(policy_id).unwrap(),
    )); // PolicyId
    fields_inner.add(&plutus::PlutusData::new_bytes(
        hex::decode(token_name).unwrap(),
    )); // TokenName

    debug!("\nFields Inner: \n{:?}\n", fields_inner);

    let stake_key_1 = &stake_key_hash[0..62];
    let stake_key_2 = &stake_key_hash[62..];

    let mut meta_list = Vec::<Vec<u8>>::new();
    meta_list.push(selling_price.as_bytes().to_vec());
    meta_list.push(hex::decode(pub_key_hash).unwrap()); // Market PubKeyHash
    meta_list.push(royalties_rate.as_bytes().to_vec()); // Royalties Rate
    meta_list.push(roy_pkey_hash); // Royalties PubKeyHash
    meta_list.push(nft_policy_id.to_bytes()); // PolicyId
    meta_list.push(nft_token_name);
    meta_list.push(hex::decode(stake_key_1).unwrap()); // StakeKeyHash
    meta_list.push(hex::decode(stake_key_2).unwrap()); // StakeKeyHash
    meta_list.push(
        format!("{}{}", "artifct-cli-", sc_version)
            .as_bytes()
            .to_vec(),
    );

    for meta in meta_list.clone() {
        debug!("\nmetalist: {:?}\n", hex::encode(meta)); //  hex::encode(selling_price.as_bytes().to_vec()));
    }

    /*
    let mut nft_shop = plutus::PlutusList::new();
    nft_shop.add (
        &plutus::PlutusData::new_constr_plutus_data(
            &plutus::ConstrPlutusData::new(
                cutils::Int::new_i32(0),
                &fields_inner
            )
        )
    );
    */
    //NFT Shop
    let datum = &plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
        &cutils::to_bignum(0),
        &fields_inner,
    ));
    debug!("NFT Shop: {:?}\n", datum.clone());
    /*
    let datum = plutus::PlutusData::new_constr_plutus_data(
                    &plutus::ConstrPlutusData::new(
                        cutils::Int::new_i32(0),
                        &nft_shop
                    )
                );
    */

    let mut datums = plutus::PlutusList::new();
    datums.add(datum);
    debug!("Datum: {:?}\n", datum.clone());
    debug!("Datums: {:?}\n", datums.clone());

    let datumhash = cutils::hash_plutus_data(datum);
    let hex_datum = hex::encode(datumhash.to_bytes());
    info!("DatumHash: {:?}\n", hex_datum);

    (datumhash, datums, meta_list)
}

#[allow(clippy::type_complexity)]
pub fn decode_datum_mp(
    metadata_datum: Vec<String>,
    net: &clib::NetworkIdKind,
) -> Result<
    (
        cutils::BigNum,
        String,
        cutils::BigNum,
        clib::PolicyID,
        String,
        plutus::PlutusList,
        ccrypto::DataHash,
        ccrypto::Ed25519KeyHash,
        String,
    ),
    MurinError,
> {
    //(ccrypto::DataHash, plutus::PlutusList) {
    let err = MurinError::new("Error in parsing Metadata into Datum");
    let m0_0 = hex::decode(metadata_datum.get(0).ok_or(&err)?)?;
    let m0 = String::from_utf8(m0_0)?; // Selling price
    let m1 = hex::decode(metadata_datum.get(1).ok_or(&err)?)?; // Seller PubKeyHash
    let rr_0 = hex::decode(metadata_datum.get(2).ok_or(&err)?)?;
    let rr = String::from_utf8(rr_0)?; // Royalty Rate
    let ra = hex::decode(metadata_datum.get(3).ok_or(&err)?)?; // Royalties PubKeyHash
    let m2 = hex::decode(metadata_datum.get(4).ok_or(&err)?)?; // Policy Id
    let m3 = hex::decode(metadata_datum.get(5).ok_or(&err)?)?; // TokenName
    let m4 = hex::decode(metadata_datum.get(6).ok_or(&err)?)?; // StakeKey 1
    let m5 = hex::decode(metadata_datum.get(7).ok_or(&err)?)?; // StakeKey 2
    let m6_0 = hex::decode(metadata_datum.get(8).ok_or(&err)?)?; // Version
    let m6 = String::from_utf8(m6_0)?; // Version

    let pub_key_hash = ccrypto::Ed25519KeyHash::from_bytes(m1.clone())?;
    let payment_credentials = caddr::StakeCredential::from_keyhash(&pub_key_hash);
    let stake_str = format!("{}{}", str::from_utf8(&m4)?, str::from_utf8(&m5)?);
    let stake_key = ccrypto::Ed25519KeyHash::from_bech32(&stake_str)?;
    let stake_credentials = caddr::StakeCredential::from_keyhash(&stake_key);
    let mut networkbyte: u8 = 0b0001;
    if *net == clib::NetworkIdKind::Testnet {
        networkbyte = 0b0000
    }
    let base_addr = caddr::BaseAddress::new(networkbyte, &payment_credentials, &stake_credentials);
    info!("Version: {}", m6);

    //Create Adress for testnet or mainnet
    let my_addr = base_addr.to_address().to_bech32(None)?;
    debug!("Reconstructed Address: {:?}", my_addr);

    let selling_price = cutils::BigNum::from_str(&m0)?;
    let trade_owner = my_addr;
    let royalties_rate = cutils::BigNum::from_str(&rr)?;
    let royalties_pkey = ccrypto::Ed25519KeyHash::from_bytes(ra.clone())?;
    let nft_policy_id = clib::PolicyID::from_bytes(m2.clone())?; // hex::encode(m2.clone()); //
                                                                 //let nft_token_name = clib::AssetName::new(m3.clone()).unwrap();
    debug!("\nRPKH: {:?} \n", hex::encode(ra.clone()));
    // ACHTUNG TO_BYTES BEI ASSETNAME PREPENDS SOME BYTES !!!

    info!(
        "\nDecode Datum: price: {:?}, PubKey Owner: {:?}, RR: {:?}, Policy: {:?}, TN: {:?}\n",
        selling_price,
        trade_owner,
        royalties_rate,
        nft_policy_id,
        hex::encode(m3.clone())
    ); // , m0,m1,m2,m3,m4); //

    let mut fields_inner = plutus::PlutusList::new();
    fields_inner.add(&plutus::PlutusData::new_integer(&cutils::BigInt::from_str(
        &m0,
    )?)); // Selling Price
    fields_inner.add(&plutus::PlutusData::new_bytes(m1)); // Sellers PubKeyHash
    fields_inner.add(&plutus::PlutusData::new_integer(&cutils::BigInt::from_str(
        &rr,
    )?)); // Royalties Rate
    fields_inner.add(&plutus::PlutusData::new_bytes(ra)); // Royalties PubKeyHash
    fields_inner.add(&plutus::PlutusData::new_bytes(m2)); // PolicyId
    fields_inner.add(&plutus::PlutusData::new_bytes(m3.clone())); // TokenName

    //NFT Shop
    let datum = &plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
        &cutils::to_bignum(0),
        &fields_inner,
    ));
    debug!("NFT Shop: {:?}\n", datum.clone());

    let mut datums = plutus::PlutusList::new();
    datums.add(datum);
    debug!("\nReconstructed Datum: {:?}\n", datum.clone());
    debug!("Datums: {:?}\n", datums.clone());

    let datumhash = cutils::hash_plutus_data(datum);
    let hex_datum = hex::encode(datumhash.to_bytes());

    info!("\nReconstructed DatumHash: {:?}\n", hex_datum);

    Ok((
        selling_price,
        trade_owner,
        royalties_rate,
        nft_policy_id,
        hex::encode(m3),
        datums,
        datumhash,
        royalties_pkey,
        m6,
    ))
}

pub fn calc_txfee(
    tx: &clib::Transaction,
    a: &cutils::BigNum,
    b: &cutils::BigNum,
    ex_unit_price: ExUnitPrice,
    steps: &cutils::BigNum,
    mem: &cutils::BigNum,
    no_sc: bool,
) -> cutils::BigNum {
    let txsfee = tx_script_fee(
        ex_unit_price,
        cutils::from_bignum(steps),
        cutils::from_bignum(mem),
    );
    let linearfee = clib::fees::LinearFee::new(a, b);
    let base_fee = clib::fees::min_fee(&tx.clone(), &linearfee).unwrap();
    let mut calculated_fee = base_fee.checked_add(&cutils::to_bignum(txsfee)).unwrap();

    if no_sc {
        calculated_fee = base_fee;
    }

    debug!("\nCalculated txs fee: {:?}", txsfee);
    debug!("Calculated base fee: {:?}", base_fee);
    info!("\nCalculated fee: {:?}\n", calculated_fee);

    calculated_fee
}

pub fn tx_script_fee(ex_unit_price: ExUnitPrice, steps: u64, mem: u64) -> u64 {
    let tx_script_fee =
        (ex_unit_price.priceMemory * mem as f64) + (ex_unit_price.priceSteps * steps as f64);
    tx_script_fee.ceil() as u64
}

#[allow(clippy::too_many_arguments)]
pub fn create_ada_tx(
    fee: &cutils::BigNum,
    dummy: bool,
    network: &clib::NetworkIdKind,
    input_utxos: TransactionUnspentOutputs,
    to: &caddr::Address,
    change: &caddr::Address,
    lovelaces: u64,
    in_current_slot: u64,
) -> Result<
    (
        clib::TransactionBody,
        clib::TransactionWitnessSet,
        clib::metadata::AuxiliaryData,
        usize,
        TransactionUnspentOutputs,
    ),
    MurinError,
> {
    if dummy {
        info!("--------------------------------------------------------------------------------------------------------");
        info!("-----------------------------------------Fee Calculation------------------------------------------------");
        info!("---------------------------------------------------------------------------------------------------------\n");
    } else {
        info!("--------------------------------------------------------------------------------------------------------");
        info!("-----------------------------------------Build Transaction----------------------------------------------");
        info!("--------------------------------------------------------------------------------------------------------\n");
    }

    /////////////////////////////////////////////////////////////////////////////////////////////////////
    //Auxiliary Data
    //  Plutus Script and Metadata
    /////////////////////////////////////////////////////////////////////////////////////////////////////
    let mut aux_data = clib::metadata::AuxiliaryData::new();
    //aux_data.set_plutus_scripts(&sc_scripts);
    let general_metadata = clib::metadata::GeneralTransactionMetadata::new();
    aux_data.set_metadata(&general_metadata);
    let aux_data_hash = cutils::hash_auxiliary_data(&aux_data);

    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //Add Inputs and Outputs
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    let mut txouts = clib::TransactionOutputs::new();
    //let (_, input_txuos) = make_inputs_txb (&input_utxos);
    txouts.add(&clib::TransactionOutput::new(
        to,
        &cutils::Value::new(&cutils::to_bignum(lovelaces)),
    ));

    let mut fee_paied = false;
    let mut first_run = true;
    let mut txos_paied = false;
    let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64)); // Inital value to balance txos
    let mut acc = cutils::Value::new(&cutils::to_bignum(0u64)); // Inital value to for change accumulator

    let mut needed_value = sum_output_values(&txouts);
    needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone())?);
    let security =
        cutils::to_bignum(cutils::from_bignum(&needed_value.coin()) / 100 * 10 + MIN_ADA); // 10% Security for min utxo etc.
    needed_value.set_coin(&needed_value.coin().checked_add(&security)?);

    debug!("Needed Value: {:?}", needed_value);

    let (txins, mut input_txuos) = input_selection(None, &mut needed_value, &input_utxos, None);
    let saved_input_txuos = input_txuos.clone();
    let vkey_counter = get_vkey_count(&input_txuos, None);

    // Balance TX
    debug!("Before Balance: Transaction Inputs: {:?}", input_txuos);
    debug!("Before Balance: Transaction Outputs: {:?}", txouts);

    let txouts_fin = balance_tx(
        &mut input_txuos,
        &Tokens::new(),
        &mut txouts,
        None,
        fee,
        &mut fee_paied,
        &mut first_run,
        &mut txos_paied,
        &mut tbb_values,
        to,
        change,
        &mut acc,
        None,
        &dummy,
    )?;

    let slot = in_current_slot + 3000;
    let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
    txbody.set_ttl(&cutils::to_bignum(slot));

    txbody.set_auxiliary_data_hash(&aux_data_hash);

    // Set network Id
    if *network == clib::NetworkIdKind::Testnet {
        txbody.set_network_id(&clib::NetworkId::testnet());
    } else {
        txbody.set_network_id(&clib::NetworkId::mainnet());
    }

    let txwitness = clib::TransactionWitnessSet::new();
    debug!("--------------------Iteration Ended------------------------------");
    Ok((txbody, txwitness, aux_data, vkey_counter, saved_input_txuos))
}
