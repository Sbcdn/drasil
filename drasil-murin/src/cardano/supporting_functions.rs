use crate::cardano::models::*;
use crate::MurinError;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto, utils as cutils};
use clib::address::BaseAddress;
use clib::address::EnterpriseAddress;
use clib::address::StakeCredKind;
use std::io::{self, BufRead};

use crate::txbuilder;
use std::env;
use std::str;

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
        return Some(txuos);
    } else {
        info!("Error: could not read stdin, {:?}", ret.status);
    }

    None
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

pub fn get_network(nws: &String) -> (clib::NetworkIdKind, &str) {
    if nws == "testnet" {
        (clib::NetworkIdKind::Testnet, "addr_test")
    } else {
        (clib::NetworkIdKind::Mainnet, "addr")
    }
}

pub fn get_network_from_address(address: &String) -> Result<clib::NetworkIdKind, MurinError> {
    let addr: caddr::Address = crate::wallet::address_from_string_non_async(address)?;
    match addr.network_id()? {
        1 => Ok(clib::NetworkIdKind::Mainnet),
        _ => Ok(clib::NetworkIdKind::Testnet),
    }
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

pub fn get_stake_keyhash(addr: &caddr::Address) -> ccrypto::Ed25519KeyHash {
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

pub fn get_payment_keyhash(addr: &caddr::Address) -> ccrypto::Ed25519KeyHash {
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
 */
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
    aux_data: Option<clib::metadata::AuxiliaryData>,
    used_utxos: String,
    royalties: u64,
    internal: bool,
) -> Result<BuildOutput, MurinError> {
    // Build and encode transaction
    let transaction = clib::Transaction::new(&txbody, &txwitness, aux_data.clone());
    let out = transaction.to_bytes();
    let tx = hex::encode(out);

    // Conserve TxWitness
    let hex_txwitness = hex::encode(txwitness.to_bytes());

    // Conserve aux data
    let hex_aux = if let Some(aux_out) = &aux_data {
        hex::encode(aux_out.to_bytes())
    } else {
        hex::encode(clib::metadata::AuxiliaryData::new().to_bytes())
    };

    // Conserve txBody in json file
    let hex_body = hex::encode(txbody.to_bytes());

    let jout = BuildOutput {
        tx_witness: hex_txwitness,
        metadata: hex_aux,
        tx_body: hex_body,
        tx_unsigned: tx,
        used_utxos,
        royalties,
        internal_transfer: internal.to_string(),
    };

    Ok(jout)
}

pub fn sum_output_values(txouts: &clib::TransactionOutputs) -> cutils::Value {
    let mut acc = cutils::Value::new(&cutils::to_bignum(64));
    for i in 0..txouts.len() {
        acc = acc.checked_add(&txouts.get(i).amount()).unwrap();
    }

    acc
}

pub fn splitt_adaonly_from_ma_utxos(
    value: cutils::Value,
    addr: caddr::Address,
    split_txos: &mut clib::TransactionOutputs,
) {
    let min_utxo_ada = txbuilder::calc_min_ada_for_utxo(&value, None);
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
                                splitt_adaonly_from_ma_utxos(value, addr, split_txos);
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
                                            splitt_adaonly_from_ma_utxos(value, addr, split_txos);
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
                                                txbuilder::calc_min_ada_for_utxo(&new_value, None);
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
                                            let min_utxo =
                                                txbuilder::calc_min_ada_for_utxo(&rest_value, None);
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
                                                    &txbuilder::calc_min_ada_for_utxo(
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
    let coins = cutils::from_bignum(&nv.coin());
    let max_coins = coins + (coins / 100 * overhead); // Coins + Overhead in %

    let mut acc = 0u64;
    let mut selection = TransactionUnspentOutputs::new();
    let mut multi_storage = TransactionUnspentOutputs::new();
    let mut coin_storage = TransactionUnspentOutputs::new();

    'outer: for tx in inputs.clone() {
        let lc = cutils::from_bignum(&tx.output().amount().coin());
        if lc > coins {
            match tx.output().amount().multiasset() {
                Some(multi) => match multi.len() {
                    0 => {
                        if lc < max_coins {
                            selection.add(&tx);
                            debug!("Selection: {:?}", selection);
                            return (Some(selection), lc);
                        } else {
                            coin_storage.add(&tx);
                        }
                    }

                    1..=21 => {
                        selection.add(&tx);
                        return (Some(selection), lc);
                    }
                    _ => {
                        multi_storage.add(&tx);
                    }
                },

                None => {
                    if lc < max_coins {
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

        let tx = coin_storage.get(0);
        selection.add(&tx);
        acc = cutils::from_bignum(&tx.output().amount().coin());
        return (Some(selection), acc);
    } else {
        for tx in inputs {
            let lc = cutils::from_bignum(&tx.output().amount().coin());
            acc += lc;
            selection.add(&tx);
            if acc > coins + MIN_ADA {
                return (Some(selection), acc);
            }
        }
    }

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

#[allow(clippy::too_many_arguments)]
pub fn balance_tx(
    input_txuos: &mut TransactionUnspentOutputs,
    _tokens: &Tokens,
    txos: &mut clib::TransactionOutputs,
    already_paid: Option<&cutils::Value>,
    fee: &cutils::BigNum,
    fee_paid: &mut bool,
    first_run: &mut bool,
    txos_paid: &mut bool,
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
            let input_address_pkey = get_stake_keyhash(&unspent_output.output().address());
            let senders_addr_pkey = get_stake_keyhash(senders_addr);
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
                if let Some(sc) = sc_addr.clone() {
                    if input_address_pkey == get_stake_keyhash(&sc) {
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
                fee_paid,
                first_run,
                txos_paid,
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
                *fee_paid = true;
                *txos_paid = true;
                debug!("------------------------------------------------------------------\n\n");
                debug!("Acc after clamped sub: \n{:?}", acc_change);
                debug!("------------------------------------------------------------------\n\n");
            }
            let min_utxo = txbuilder::calc_min_ada_for_utxo(acc_change, None);

            let min_ada_value = cutils::Value::new(&min_utxo);
            if *txos_paid && *fee_paid && acc_change.coin().compare(&min_ada_value.coin()) >= 0 {
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
            if (!*txos_paid
                || !*fee_paid
                || acc_change.coin().compare(&cutils::to_bignum(0u64)) != 0
                || acc_change.multiasset().is_some())
                && !*dummyrun
            {
                let mut overhead = &mut cutils::Value::new(&cutils::to_bignum(0u64));
                if !*txos_paid {
                    overhead = tbb_values;
                }
                if !*fee_paid {
                    overhead.set_coin(&overhead.coin().checked_add(fee).unwrap());
                }
                panic!("\nERROR: Transaction does not balance: {overhead:?} overhead, fee paid: {fee_paid:?}, outputs paid: {txos_paid:?}");
            }
            debug!("Accumulated Change is Zero?: {:?}", acc_change);
            Ok(txos.clone())
        }
    }
}

pub fn calc_txfee(
    tx: &clib::Transaction,
    a: &cutils::BigNum,
    b: &cutils::BigNum,
    ex_unit_price: ExUnitPrice,
    steps: &cutils::BigNum,
    mem: &cutils::BigNum,
    sc: bool,
) -> cutils::BigNum {
    let txsfee = tx_script_fee(
        ex_unit_price,
        cutils::from_bignum(steps),
        cutils::from_bignum(mem),
    );
    let linearfee = clib::fees::LinearFee::new(a, b);
    let base_fee = clib::fees::min_fee(&tx.clone(), &linearfee).unwrap();
    let mut calculated_fee = base_fee.checked_add(&cutils::to_bignum(txsfee)).unwrap();

    if !sc {
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
#[deprecated]
/// depricated function to create an Ada only transaction, use Standard Transaction instead
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

    let mut fee_paid = false;
    let mut first_run = true;
    let mut txos_paid = false;
    let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64)); // Inital value to balance txos
    let mut acc = cutils::Value::new(&cutils::to_bignum(0u64)); // Inital value to for change accumulator

    let mut needed_value = sum_output_values(&txouts);
    needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone())?);
    let security =
        cutils::to_bignum(cutils::from_bignum(&needed_value.coin()) / 100 * 10 + MIN_ADA); // 10% Security for min utxo etc.
    needed_value.set_coin(&needed_value.coin().checked_add(&security)?);

    debug!("Needed Value: {:?}", needed_value);

    let (txins, mut input_txuos) =
        crate::input_selection(None, &mut needed_value, &input_utxos, None, None)?;
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
        &mut fee_paid,
        &mut first_run,
        &mut txos_paid,
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
