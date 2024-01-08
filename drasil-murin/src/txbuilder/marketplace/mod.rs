use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto, utils as cutils};
use clib::crypto::Ed25519KeyHash;
use clib::{plutus, AssetName, PolicyID};

pub mod buy;
pub mod cancel;
pub mod list;
pub mod update;

use self::models::TxInput;

pub use super::*;
pub use buy::*;
pub use cancel::*;
pub use list::*;
pub use update::*;

#[derive(Debug, Clone)]
pub struct MpTxData {
    tokens: Vec<TokenAsset>,
    token_utxos: TransactionUnspentOutputs,
    royalties_addr: Option<caddr::Address>,
    royalties_rate: Option<f32>,
    selling_price: u64,
}

impl ToString for MpTxData {
    fn to_string(&self) -> String {
        // prepare tokens vector
        let mut s_tokens = String::new();
        for ta in self.get_tokens() {
            let mut subs = String::new();
            subs.push_str(&(hex::encode(ta.0.to_bytes()) + "?"));
            subs.push_str(&(hex::encode(ta.1.to_bytes()) + "?"));
            subs.push_str(&(hex::encode(ta.2.to_bytes()) + "!"));
            s_tokens.push_str(&subs);
        }
        // erase last !
        s_tokens.pop();

        // prepare token_utxos
        let s_token_utxos = match self.get_token_utxos().to_hex() {
            Ok(s) => s,
            _ => "NoData".to_string(),
        };

        // prepare royalties address
        let s_royaddr = match self.get_royalties_address() {
            Some(a) => hex::encode(a.to_bytes()),
            None => "NoData".to_string(),
        };

        // prepare royalties rate
        let s_royrate = match self.get_royalties_rate() {
            Some(a) => a.to_string(),
            None => "NoData".to_string(),
        };

        // prepare selling price
        let s_sprice = self.get_price().to_string();

        let mut ret = String::new();
        ret.push_str(&(s_tokens + "|"));
        ret.push_str(&(s_token_utxos + "|"));
        ret.push_str(&(s_royaddr + "|"));
        ret.push_str(&(s_royrate + "|"));
        ret.push_str(&(s_sprice));
        ret
    }
}

//TODO
impl std::str::FromStr for MpTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let slice: Vec<&str> = src.split('|').collect();
        if slice.len() == 6 {
            // restore token vector
            let mut tokens = Vec::<TokenAsset>::new();
            let tokens_vec: Vec<&str> = slice[0].split('!').collect();
            for token in tokens_vec {
                let token_slice: Vec<&str> = token.split('?').collect();
                tokens.push((
                    clib::PolicyID::from_bytes(hex::decode(token_slice[0])?)?,
                    clib::AssetName::from_bytes(hex::decode(token_slice[1])?)?,
                    cutils::BigNum::from_bytes(hex::decode(token_slice[2])?)?,
                ))
            }
            debug!("Tokens: {:?}", tokens);

            // restore token_utxos
            let token_utxos = match slice[1] {
                "NoData" => {
                    return Err(MurinError::new(
                        "Error: No Tokens for sale in marketplace transaction data",
                    ))
                }
                _ => TransactionUnspentOutputs::from_hex(slice[1])?,
            };

            // restore royalties addr
            let roy_addr = match slice[2] {
                "NoData" => None,
                _ => Some(caddr::Address::from_bytes(hex::decode(slice[2])?)?),
            };

            // restore roy rate
            let roy_rate = match slice[3] {
                "NoData" => None,
                _ => Some(slice[3].parse::<f32>()?),
            };

            // restore selling price
            let selling_price = slice[4].parse::<u64>()?;

            Ok(MpTxData {
                tokens,
                token_utxos,
                royalties_addr: roy_addr,
                royalties_rate: roy_rate,
                selling_price,
            })
        } else {
            Err(MurinError::new(
                //std::io::Error::new(
                //    std::io::ErrorKind::InvalidData,
                &format!("Error the provided string '{src}' cannot be parsed into 'MpTxData' ",),
            ))
        }
    }
}

impl MpTxData {
    pub fn new(
        tokens: Vec<TokenAsset>,
        token_utxos: TransactionUnspentOutputs,
        selling_price: u64,
    ) -> MpTxData {
        MpTxData {
            tokens,
            token_utxos,
            royalties_addr: None,
            royalties_rate: None,
            selling_price,
        }
    }

    pub fn set_royalties_address(&mut self, royaddr: caddr::Address) {
        self.royalties_addr = Some(royaddr);
    }

    pub fn set_royalties_rate(&mut self, royrate: f32) {
        self.royalties_rate = Some(royrate);
    }

    pub fn get_tokens(&self) -> &Vec<TokenAsset> {
        &self.tokens
    }

    pub fn get_token_utxos(&self) -> TransactionUnspentOutputs {
        self.token_utxos.clone()
    }

    pub fn get_royalties_address(&self) -> Option<caddr::Address> {
        self.royalties_addr.clone()
    }

    pub fn get_royalties_rate(&self) -> Option<f32> {
        self.royalties_rate
    }

    pub fn get_price(&self) -> u64 {
        self.selling_price
    }
}

pub fn make_mp_contract_utxo_output(
    txos: &mut clib::TransactionOutputs,
    receiver: caddr::Address,
    datum: &ccrypto::DataHash,
    tokens: &Vec<TokenAsset>,
    //token_utxos : &TransactionUnspentOutputs,
    set_datum: bool,
) -> Option<clib::TransactionOutputs> {
    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //
    //Add Script Outputs for Transaction Body
    //  Add the NFT from the smart contract
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    //let mut txuos = TransactionUnspentOutputs::new();
    // let mut txuo : Option<cutils::TransactionUnspentOutput> = None;

    if !tokens.is_empty() {
        let mut value = cutils::Value::new(&cutils::to_bignum(0u64));
        //let mut lovelaces : u64 = 0;
        let mut multiasset = clib::MultiAsset::new();

        for token in tokens {
            let mut assets = clib::Assets::new();
            assets.insert(&token.1, &token.2);
            multiasset.insert(&token.0, &assets);
        }
        value.set_multiasset(&multiasset);

        // Todo calc min ada needs to go to txbuilders module
        let min_ada_utxo = crate::calc_min_ada_for_utxo(&value, Some(datum.clone()));
        value.set_coin(&min_ada_utxo);
        let mut txout = clib::TransactionOutput::new(&receiver, &value);
        //let txin  = clib::TransactionInput::new(
        //    &ccrypto::TransactionHash::from_bytes(hex::decode(o.txhash.clone()).unwrap()).unwrap(),o.txinput);

        if set_datum {
            txout.set_data_hash(datum)
        };
        txos.add(&txout);

        Some(txos.clone())
    } else {
        None
    }
}

pub struct MarketPlaceDatum {
    price: u64,
    seller: Ed25519KeyHash,
    royalties_rate: u64,
    royalties_pkh: Option<Ed25519KeyHash>,
    policy_id: PolicyID,
    token_name: AssetName,
}

pub fn encode_mp_datum(mp: MarketPlaceDatum) -> (ccrypto::DataHash, plutus::PlutusData) {
    let roy_rate: u64;
    let roy_pkey_hash = if let Some(rpkh) = &mp.royalties_pkh {
        roy_rate = mp.royalties_rate;
        rpkh.to_bytes()
    } else {
        roy_rate = 0;
        vec![]
    };

    let mut fields_inner = plutus::PlutusList::new();
    // Selling Price
    fields_inner.add(&plutus::PlutusData::new_integer(
        &cutils::BigInt::from_str(&mp.price.to_string()).unwrap(),
    ));
    // Sellers PubKeyHash
    fields_inner.add(&plutus::PlutusData::new_bytes(
        hex::decode(&mp.seller.to_bytes()).unwrap(),
    ));
    // royalties rate in promille
    fields_inner.add(&plutus::PlutusData::new_integer(
        &cutils::BigInt::from_str(&roy_rate.to_string()).unwrap(),
    ));
    // Royalties PubKeyHash
    fields_inner.add(&plutus::PlutusData::new_bytes(roy_pkey_hash));
    // PolicyId
    fields_inner.add(&plutus::PlutusData::new_bytes(
        hex::decode(&mp.policy_id.to_bytes()).unwrap(),
    ));
    // TokenName
    fields_inner.add(&plutus::PlutusData::new_bytes(
        hex::decode(&mp.token_name.name()).unwrap(),
    ));

    debug!("\nFields Inner: \n{:?}\n", fields_inner);

    //Datum
    let datum = &plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
        &cutils::to_bignum(0),
        &fields_inner,
    ));
    // Datumhash
    let datumhash = cutils::hash_plutus_data(datum);
    let hex_datum = hex::encode(datumhash.to_bytes());
    info!("DatumHash: {:?}\n", hex_datum);

    (datumhash, datum.clone())
}

pub fn decode_mp_datum(bytes: &[u8]) -> Result<MarketPlaceDatum, MurinError> {
    let datum = cardano_serialization_lib::plutus::PlutusData::from_bytes(bytes.to_vec())
        .expect("Could not deserialize PlutusData");
    log::debug!("Restored PlutusData: {:?}", datum);
    let d_str = datum
        .to_json(cardano_serialization_lib::plutus::PlutusDatumSchema::DetailedSchema)
        .expect("Could not transform PlutusData to JSON");
    log::info!("Restored PlutusData Str: {:?}", d_str);
    let d_svalue = serde_json::from_str::<serde_json::Value>(&d_str)
        .expect("Could not transform PlutusDataJson to serde_json::Value");
    log::debug!("Deserialized Datum: \n{:?}", &d_str);
    let fields = d_svalue.get("fields").unwrap().as_array().unwrap();

    let price = fields[0].as_u64().unwrap();

    let seller = Ed25519KeyHash::from_bytes(
        hex::decode(
            fields[1]
                .as_object()
                .unwrap()
                .get("bytes")
                .unwrap()
                .as_str()
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();

    let royalties_rate = fields[2].as_u64().unwrap();

    let royalties_pkh = Ed25519KeyHash::from_bytes(
        hex::decode(
            fields[3]
                .as_object()
                .unwrap()
                .get("bytes")
                .unwrap()
                .as_str()
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();

    let policy_id = PolicyID::from_hex(
        fields[4]
            .as_object()
            .unwrap()
            .get("bytes")
            .unwrap()
            .as_str()
            .unwrap(),
    )
    .unwrap();

    let token_name = AssetName::new(
        hex::decode(
            fields[5]
                .as_object()
                .unwrap()
                .get("bytes")
                .unwrap()
                .as_str()
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();

    Ok(MarketPlaceDatum {
        price,
        seller,
        royalties_rate,
        royalties_pkh: Some(royalties_pkh),
        policy_id,
        token_name,
    })
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
