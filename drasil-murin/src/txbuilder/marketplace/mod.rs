use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto, utils as cutils};

pub mod buy;
pub mod cancel;
pub mod list;
pub mod update;

pub use super::*;
pub use buy::*;
pub use cancel::*;
pub use list::*;
pub use update::*;

/// Marketplace transaction data. This corresponds to `Operation::Marketplace` in Hugin,
/// except that the UTxO:s are filtered to only include those that contain relevant tokens.
#[derive(Debug, Clone)]
pub struct MpTxData {
    tokens: Vec<TokenAsset>,
    token_utxos: TransactionUnspentOutputs,
    royalties_addr: Option<caddr::Address>,
    royalties_rate: Option<f32>,
    selling_price: u64,
    metadata: Option<Vec<String>>,
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

        //prepare metadata
        let mut s_metadata = String::new();
        match self.get_metadata() {
            Some(m) => {
                for s in m {
                    s_metadata.push_str(&(s + "?"))
                }
                s_metadata.pop();
            }
            None => {
                s_metadata = "NoData".to_string();
            }
        }

        let mut ret = String::new();
        ret.push_str(&(s_tokens + "|"));
        ret.push_str(&(s_token_utxos + "|"));
        ret.push_str(&(s_royaddr + "|"));
        ret.push_str(&(s_royrate + "|"));
        ret.push_str(&(s_sprice + "|"));
        ret.push_str(&(s_metadata));

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

            // restore metadata
            let metadata = match slice[4] {
                "NoData" => None,
                _ => {
                    let mut md = Vec::<String>::new();
                    let meta_slice: Vec<&str> = slice[5].split('?').collect();
                    for d in meta_slice {
                        md.push(d.to_string());
                    }
                    Some(md)
                }
            };

            Ok(MpTxData {
                tokens,
                token_utxos,
                royalties_addr: roy_addr,
                royalties_rate: roy_rate,
                selling_price,
                metadata,
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
    /// Instantiate marketplace transaction data. 
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
            metadata: None,
        }
    }

    pub fn set_royalties_address(&mut self, royaddr: caddr::Address) {
        self.royalties_addr = Some(royaddr);
    }

    pub fn set_royalties_rate(&mut self, royrate: f32) {
        self.royalties_rate = Some(royrate);
    }

    pub fn set_metadata(&mut self, metadata: Vec<String>) {
        self.metadata = Some(metadata);
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

    pub fn get_metadata(&self) -> Option<Vec<String>> {
        self.metadata.clone()
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
