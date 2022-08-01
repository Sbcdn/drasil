/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use murin::{address::Address, TxData};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Error, str::FromStr};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ContractType {
    MarketPlace,
    NftShop,
    NftMinter,
    TokenMinter,
    DrasilAPILiquidity,
    Other,
}

impl ContractType {
    pub const CONTRTYPES: [Self; 4] = [
        Self::MarketPlace,
        Self::NftShop,
        Self::NftMinter,
        Self::TokenMinter,
    ];
}

impl FromStr for ContractType {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "mp" => Ok(ContractType::MarketPlace),
            "nftshop" => Ok(ContractType::NftShop),
            "nftmint" => Ok(ContractType::NftMinter),
            "tokmint" => Ok(ContractType::TokenMinter),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Contract Type {} does not exist", src),
            )),
        }
    }
}

impl ToString for ContractType {
    fn to_string(&self) -> String {
        match &self {
            &Self::MarketPlace => "mp".to_string(),
            &Self::NftShop => "nftshop".to_string(),
            &Self::NftMinter => "nftmint".to_string(),
            &Self::TokenMinter => "tokmint".to_string(),
            &Self::DrasilAPILiquidity => "drasilliquidity".to_string(),
            &Self::Other => "not implemented".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MarketplaceActions {
    List,
    Buy,
    Cancel,
    Update,
}

impl MarketplaceActions {
    pub const MRKTACTIONS: [Self; 4] = [Self::List, Self::Buy, Self::Cancel, Self::Update];
}

impl FromStr for MarketplaceActions {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "list" => Ok(MarketplaceActions::List),
            "buy" => Ok(MarketplaceActions::Buy),
            "cancel" => Ok(MarketplaceActions::Cancel),
            "update" => Ok(MarketplaceActions::Update),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Marketplace Action {} does not exist", src),
            )),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MultiSigType {
    SpoRewardClaim,
    NftVendor,
    DAOVoting,
    VestingWallet,
    Mint,
    ClAPIOneShotMint,
    TestRewards,
    Other,
    CustomerPayout,
}

impl MultiSigType {
    // ToDo: Check
    pub const MULTISIGTYPES: [Self; 2] = [Self::SpoRewardClaim, Self::NftVendor];
}

impl FromStr for MultiSigType {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "sporwc" => Ok(MultiSigType::SpoRewardClaim),
            "nvendor" => Ok(MultiSigType::NftVendor),
            "mint" => Ok(MultiSigType::Mint),
            "clapioneshotmint" => Ok(MultiSigType::ClAPIOneShotMint),
            "testrewards" => Ok(MultiSigType::TestRewards),
            "cpo" => Ok(MultiSigType::CustomerPayout),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Transaction Type {} does not exist", src),
            )),
        }
    }
}

impl ToString for MultiSigType {
    fn to_string(&self) -> String {
        match &self {
            &Self::SpoRewardClaim => "sporwc".to_string(),
            &Self::NftVendor => "nvendor".to_string(),
            &Self::DAOVoting => "dvotng".to_string(),
            &Self::VestingWallet => "vesting".to_string(),
            &Self::Mint => "mint".to_string(),
            &Self::ClAPIOneShotMint => "clapioneshotmint".to_string(),
            &Self::TestRewards => "testrewards".to_string(),
            &Self::CustomerPayout => "cpo".to_string(),
            &Self::Other => "not implemented".to_string(),
        }
    }
}

/*
// Depricated
#[derive(Serialize, Deserialize,Debug,Clone)]
pub enum MultiSigAction {

}
*/

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StdTxType {
    DelegateStake,
}

impl FromStr for StdTxType {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "stakedelegation" => Ok(StdTxType::DelegateStake),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Transaction Type {} does not exist", src),
            )),
        }
    }
}

impl ToString for StdTxType {
    fn to_string(&self) -> String {
        match &self {
            &Self::DelegateStake => "stakedelegation".to_string(),
        }
    }
}

/*
// Depricated

#[derive(Serialize, Deserialize,Debug)]
pub enum PayloadWrapper {
    U64(u64),
    MultiSigType(MultiSigType),
    ContractType(ContractType),
    TransactionPattern(TransactionPattern),
    Signature(Signature),
    TransactionId(String),
}
 */

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signature {
    signature: String,
}

impl Signature {
    pub fn get_signature(&self) -> String {
        self.signature.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ContractAction {
    MarketplaceActions(MarketplaceActions),
}

impl ContractAction {}

impl FromStr for ContractAction {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "list" => Ok(ContractAction::MarketplaceActions(MarketplaceActions::List)),
            "buy" => Ok(ContractAction::MarketplaceActions(MarketplaceActions::Buy)),
            "cancel" => Ok(ContractAction::MarketplaceActions(
                MarketplaceActions::Cancel,
            )),
            "update" => Ok(ContractAction::MarketplaceActions(
                MarketplaceActions::Update,
            )),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                format!("ContractAction '{}' does not exist", src),
            )),
        }
    }
}

impl ToString for ContractAction {
    fn to_string(&self) -> String {
        match &self {
            &Self::MarketplaceActions(MarketplaceActions::List) => "list".to_string(),
            &Self::MarketplaceActions(MarketplaceActions::Buy) => "buy".to_string(),
            &Self::MarketplaceActions(MarketplaceActions::Cancel) => "cancel".to_string(),
            &Self::MarketplaceActions(MarketplaceActions::Update) => "update".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinterToken {
    tokenname: String,
    amount: u64,
}

impl MinterToken {
    pub fn into_mintasset(&self) -> Result<murin::txbuilders::MintTokenAsset, murin::MurinError> {
        let tn = murin::chelper::string_to_assetname(&self.tokenname)?;
        let amt = murin::chelper::u64_to_bignum(self.amount);
        Ok((None, tn, amt))
    }

    pub fn for_all_into_mintasset(
        ut: &Vec<MinterToken>,
    ) -> Result<Vec<murin::txbuilders::MintTokenAsset>, murin::MurinError> {
        let mut out = Vec::<murin::txbuilders::MintTokenAsset>::new();
        for t in ut {
            out.push(t.into_mintasset()?)
        }
        Ok(out)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    tokenname: String,
    currencysymbol: String,
    fingerprint: Option<String>,
    amount: u64,
}

impl Token {
    pub fn into_asset(&self) -> Result<murin::txbuilders::TokenAsset, murin::MurinError> {
        let cs = murin::chelper::string_to_policy(&self.currencysymbol)?;
        let tn = murin::chelper::string_to_assetname(&self.tokenname)?;
        let amt = murin::chelper::u64_to_bignum(self.amount);
        Ok((cs, tn, amt))
    }

    pub fn for_all_into_asset(
        ut: &Vec<Token>,
    ) -> Result<Vec<murin::txbuilders::TokenAsset>, murin::MurinError> {
        let mut out = Vec::<murin::txbuilders::TokenAsset>::new();
        for t in ut {
            out.push(t.into_asset()?)
        }
        Ok(out)
    }
}

pub type Value = HashMap<String, HashMap<String, u64>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Utxo {
    // ToDo: implement conversion function to ser.lib UnspentTransactionOutput
    tx_hash: String,
    tx_index: u64,
    value: Value,
    datum_hash: String,
    address: String,
}

pub type Utxos = Vec<Utxo>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Network {
    Testnet,
    Mainnet,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionPattern {
    user: String,
    contract_id: Option<u64>,
    wallet_type: Option<WalletType>, // yoroi, ccvault, gero, flint, ... // or yoroi, cip30, typhon
    sending_wal_addrs: Vec<String>,
    sending_stake_addr: Option<String>,
    outputs: Option<Vec<String>>,
    inputs: Option<Vec<String>>,
    excludes: Option<Vec<String>>,
    collateral: Option<Vec<String>>,
    script: ScriptSpecParams,
    network: u64,
}

impl TransactionPattern {
    pub fn new_empty(customer_id: u64, script_spec: &ScriptSpecParams, network: u64) -> Self {
        TransactionPattern {
            user: customer_id.to_string(),
            contract_id: None,
            wallet_type: None,
            sending_wal_addrs: Vec::<String>::new(),
            sending_stake_addr: None,
            outputs: None,
            inputs: Some(Vec::<String>::new()),
            excludes: None,
            collateral: None,
            script: script_spec.clone(),
            network: network,
        }
    }

    pub fn user(&self) -> String {
        self.user.clone()
    }

    pub fn contract_id(&self) -> Option<u64> {
        self.contract_id
    }

    pub fn wallet_type(&self) -> Option<WalletType> {
        self.wallet_type.clone()
    }

    pub fn sending_wal_addrs(&self) -> Vec<String> {
        self.sending_wal_addrs.clone()
    }

    pub fn set_sending_wal_addrs(&mut self, vec: &Vec<String>) -> () {
        self.sending_wal_addrs = vec.clone();
    }

    pub fn sending_stake_addr(&self) -> Option<String> {
        self.sending_stake_addr.clone()
    }

    pub fn outputs(&self) -> Option<Vec<String>> {
        self.outputs.clone()
    }

    pub fn inputs(&self) -> Option<Vec<String>> {
        self.inputs.clone()
    }

    pub fn excludes(&self) -> Option<Vec<String>> {
        self.excludes.clone()
    }

    pub fn collateral(&self) -> Option<String> {
        match &self.collateral {
            Some(col) => {
                if col.len() > 0 {
                    Some(col[0].clone())
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn network(&self) -> u64 {
        self.network
    }

    pub fn script(&self) -> Option<ScriptSpecParams> {
        Some(self.script.clone())
    }

    pub async fn into_txdata(&self) -> Result<murin::txbuilders::TxData, murin::error::MurinError> {
        let inputs = match self.inputs() {
            None => {
                return Err(murin::error::MurinError::new(
                    "Cannot build transaction data, no inputs provided",
                ))
            }
            Some(data) => data,
        };

        let saddr = match self.sending_stake_addr() {
            Some(sa) => match murin::wallet::decode_addr(&sa).await {
                Ok(addr) => Some(addr),
                Err(_) => None,
            },
            None => None,
        };

        // This is a data library and we should not make a database connection here
        //let dbsync = mimir::establish_connection()?;
        //let slot = mimir::get_slot(&dbsync)?;
        let mut txd = TxData::new(
            self.contract_id(),
            murin::wallet::decode_addresses(&self.sending_wal_addrs()).await?,
            saddr,
            murin::wallet::get_transaction_unspent_outputs(
                inputs.as_ref(),
                self.collateral().as_ref(),
                self.excludes().as_ref(),
            )
            .await?,
            murin::wallet::get_network_kind(self.network).await?,
            0u64, //slot as u64, //Is set int build contract / build multisig
        )?;

        if let Some(outputs) = self.outputs() {
            txd.set_outputs(
                murin::wallet::get_transaction_unspent_outputs(&outputs, None, None).await?,
            )
        }

        if let Some(collateral) = self.collateral() {
            txd.set_collateral(murin::wallet::get_transaction_unspent_output(&collateral).await?)
        }

        if let Some(excludes) = self.excludes() {
            txd.set_excludes(
                murin::wallet::get_transaction_unspent_outputs(&excludes, None, None).await?,
            )
        }

        Ok(txd)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ScriptSpecParams {
    SpoRewardClaim {
        reward_tokens: Vec<Token>,
        recipient_stake_addr: String,
        recipient_payment_addr: String,
        //reward_wallet_addr      : String,
        //reward_utxos            : Option<Vec::<String>>,
    },
    NftVendor {},
    Marketplace {
        tokens: Vec<Token>,
        metadata: Vec<String>,
        royalties_addr: Option<String>,
        royalties_rate: Option<f32>,
        selling_price: u64,
    },
    NftShop {
        tokens: Vec<Token>,
        metadata: Vec<String>,
        selling_price: u64,
    },
    NftMinter {
        mint_tokens: Option<Vec<MinterToken>>,
        receiver_stake_addr: Option<String>,
        receiver_payment_addr: String,
        mint_metadata: Option<String>,
        auto_mint: Option<bool>,
        contract_id: i64,
    },
    TokenMinter {},
    NftOffer {
        token: Token,
        token_owner_addr: String,
        metadata: Vec<String>,
        royalties_addr: Option<String>,
        royalties_rate: Option<f32>,
        offer_price: u64,
    },
    Auction {},
    StakeDelegation {
        poolhash: String,
    },
    //CustomerPayout
    CPO {
        contract_id: i64,
        user_id: i64,
        security_code: String,
    },
    ClApiOneShotMint {
        tokennames: Vec<String>,
        amounts: Vec<u64>,
        metadata: murin::minter::Cip25Metadata,
        receiver: String,
    },
}

impl ScriptSpecParams {
    pub async fn into_mp(
        &self,
        avail_inputs: murin::TransactionUnspentOutputs,
    ) -> Result<murin::txbuilders::marketplace::MpTxData, murin::error::MurinError> {
        use murin::error::MurinError;
        use murin::txbuilders::marketplace::MpTxData;

        match self {
            ScriptSpecParams::Marketplace {
                tokens,
                metadata,
                royalties_addr,
                royalties_rate,
                selling_price,
            } => {
                let assets = Token::for_all_into_asset(tokens)?;
                let token_utxos =
                    murin::txbuilders::find_token_utxos(avail_inputs, assets.clone()).await?;

                // Transform tokens to TokenAsset (flattend Value)

                let mut mptx = MpTxData::new(assets, token_utxos, *selling_price);

                if let Some(royaddr) = royalties_addr {
                    mptx.set_royalties_address(murin::decode_addr(&royaddr).await?);
                }

                if let Some(royrate) = royalties_rate {
                    mptx.set_royalties_rate(*royrate);
                }

                if !metadata.is_empty() {
                    mptx.set_metadata(metadata.clone());
                }

                Ok(mptx)
            }
            _ => Err(MurinError::new(
                "provided wrong specfic paramter for this contract",
            )),
        }
    }

    pub async fn into_rwd(
        &self,
    ) -> Result<murin::txbuilders::rwdist::RWDTxData, murin::error::MurinError> {
        use murin::error::MurinError;
        use murin::txbuilders::rwdist::RWDTxData;

        match self {
            ScriptSpecParams::SpoRewardClaim {
                reward_tokens,
                recipient_stake_addr,
                recipient_payment_addr,
                // reward_wallet_addr,
                // reward_utxos,
            } => {
                let assets = Token::for_all_into_asset(reward_tokens)?;
                let stake_addr = murin::decode_addr(&recipient_stake_addr).await?;
                let payment_addr = murin::decode_addr(&recipient_payment_addr).await?;
                // let rwd_addr = murin::decode_addr(&reward_wallet_addr).await?;
                // let mut utxos = murin::TransactionUnspentOutputs::new();
                // if let Some(rwdutxo) = reward_utxos {
                //     utxos = murin::wallet::get_transaction_unspent_outputs(&rwdutxo, None, None).await?;
                // }

                // let rwd_utxos = if utxos.is_empty() {
                //     None
                // } else {
                //     Some(utxos)
                // };

                Ok(RWDTxData::new(
                    &assets,
                    &stake_addr,
                    &payment_addr,
                    //       &rwd_addr,
                    //       &rwd_utxos
                ))
            }
            _ => Err(MurinError::new(
                "provided wrong specfic paramter for this contract",
            )),
        }
    }

    pub async fn into_mintdata(
        &self,
    ) -> Result<murin::txbuilders::minter::MinterTxData, murin::error::MurinError> {
        use murin::error::MurinError;
        use murin::txbuilders::minter::MinterTxData;

        match self {
            ScriptSpecParams::NftMinter {
                mint_tokens,
                receiver_stake_addr,
                receiver_payment_addr,
                mint_metadata,
                auto_mint,
                contract_id,
            } => {
                let assets = match mint_tokens {
                    Some(tokens) => MinterToken::for_all_into_mintasset(tokens)?,
                    None => Vec::<murin::MintTokenAsset>::new(),
                };
                let stake_addr = match receiver_stake_addr {
                    Some(addr) => Some(murin::decode_addr(&addr).await?),
                    None => None,
                };
                let payment_addr = murin::decode_addr(&receiver_payment_addr).await?;
                let metadata = match mint_metadata {
                    Some(data) => {
                        if *data != "".to_string() {
                            log::debug!("Serde deserializing script parameter");
                            serde_json::from_str(&data)?
                        } else {
                            murin::minter::Cip25Metadata::new()
                        }
                    }
                    None => murin::minter::Cip25Metadata::new(),
                };

                let am = match auto_mint {
                    Some(data) => *data,
                    None => false,
                };
                Ok(MinterTxData::new(
                    assets,
                    stake_addr,
                    payment_addr,
                    metadata,
                    am,
                    None,
                    None,
                    *contract_id,
                ))
            }
            ScriptSpecParams::ClApiOneShotMint {
                tokennames,
                amounts,
                metadata,
                receiver,
            } => {
                let mut assets = Vec::<murin::txbuilders::MintTokenAsset>::new();
                for (i, t) in tokennames.iter().enumerate() {
                    let tn = murin::chelper::string_to_assetname(&hex::encode(t.as_bytes()))?;
                    let amt = murin::chelper::u64_to_bignum(amounts[i]);
                    assets.push((None, tn, amt))
                }
                let payment_addr = murin::b_decode_addr(&receiver).await?;
                Ok(MinterTxData::new(
                    assets,
                    None,
                    payment_addr,
                    metadata.to_owned(),
                    false,
                    None,
                    None,
                    -1,
                ))
            }
            _ => Err(MurinError::new(
                "provided wrong specfic paramter for this contract",
            )),
        }
    }

    pub async fn into_stake_delegation(
        &self,
    ) -> Result<murin::txbuilders::delegation::DelegTxData, murin::error::MurinError> {
        use murin::error::MurinError;
        use murin::txbuilders::delegation::DelegTxData;

        match self {
            ScriptSpecParams::StakeDelegation { poolhash } => Ok(DelegTxData::new(poolhash)?),
            _ => Err(MurinError::new(
                "provided wrong specfic paramter for this transaction",
            )),
        }
    }

    pub async fn into_cpo(&self) -> Result<murin::txbuilders::CPO, murin::error::MurinError> {
        use murin::error::MurinError;
        use murin::txbuilders::CPO;

        match self {
            ScriptSpecParams::CPO {
                contract_id,
                user_id,
                security_code,
            } => Ok(CPO::new(user_id, contract_id, security_code)),

            _ => Err(MurinError::new(
                "provided wrong specfic paramter for this transaction",
            )),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum WalletType {
    Nami,
    Eternl,
    Gero,
    Flint,
    Yoroi,
    Typhon,
}

impl FromStr for WalletType {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "nami" => Ok(WalletType::Nami),
            "gero" => Ok(WalletType::Gero),
            "ccvault" => Ok(WalletType::Eternl),
            "flint" => Ok(WalletType::Flint),
            "yoroi" => Ok(WalletType::Yoroi),
            "typhon" => Ok(WalletType::Typhon),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Wallet '{}' not supportet or wrong typed input", src),
            )),
        }
    }
}

impl ToString for WalletType {
    fn to_string(&self) -> String {
        match &self {
            &Self::Nami => "nami".to_string(),
            &Self::Eternl => "gero".to_string(),
            &Self::Gero => "ccvault".to_string(),
            &Self::Flint => "flint".to_string(),
            &Self::Yoroi => "yoroi".to_string(),
            &Self::Typhon => "typhon".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReturnError {
    pub msg: String,
}

impl ReturnError {
    pub fn new(str: &String) -> ReturnError {
        ReturnError { msg: str.clone() }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UnsignedTransaction {
    id: String,
    tx: String,
}

impl UnsignedTransaction {
    pub fn new(tx: Option<&String>, id: &String) -> UnsignedTransaction {
        match tx {
            Some(s) => UnsignedTransaction {
                tx: s.to_string(),
                id: id.to_string(),
            },
            None => UnsignedTransaction {
                tx: "".to_string(),
                id: id.to_string(),
            },
        }
    }

    pub fn get_tx(&self) -> String {
        self.tx.clone()
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    pub fn set_tx(&mut self, s: &String) -> () {
        self.tx = s.to_string();
    }

    pub fn set_id(&mut self, s: &String) -> () {
        self.id = s.to_string();
    }
}

impl ToString for UnsignedTransaction {
    fn to_string(&self) -> String {
        format!("{}|{}", self.id, self.tx)
    }
}

impl FromStr for UnsignedTransaction {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let slice: Vec<&str> = src.split("|").collect();
        if slice.len() != 2 {
            Err(Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{}", src),
            ))
        } else {
            Ok(UnsignedTransaction {
                id: slice[0].to_string(),
                tx: slice[1].to_string(),
            })
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TxHash {
    txhash: String,
}

impl TxHash {
    pub fn new(str: &String) -> TxHash {
        TxHash {
            txhash: str.to_string(),
        }
    }

    pub fn set_txhash(&mut self, str: &String) -> () {
        self.txhash = str.to_string();
    }
}

// Client API Types
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OneShotMintPayload {
    tokennames: Vec<String>,
    amounts: Vec<u64>,
    metadata: murin::minter::Cip25Metadata,
    receiver: String,
    network: u8,
}

impl OneShotMintPayload {
    pub fn into_script_spec(&self) -> ScriptSpecParams {
        ScriptSpecParams::ClApiOneShotMint {
            tokennames: self.tokennames.to_owned(),
            amounts: self.amounts.to_owned(),
            metadata: self.metadata.to_owned(),
            receiver: self.receiver.to_owned(),
        }
    }

    pub fn tokennames(&self) -> Vec<String> {
        self.tokennames.clone()
    }

    pub fn amounts(&self) -> Vec<u64> {
        self.amounts.clone()
    }

    pub fn metadata(&self) -> murin::minter::Cip25Metadata {
        self.metadata.clone()
    }

    pub fn receiver(&self) -> String {
        self.receiver.clone()
    }

    pub fn network(&self) -> u64 {
        self.network as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneShotReturn {
    policy_id: String,
    tokennames: Vec<String>,
    amounts: Vec<u64>,
    txhash: String,
    metadata: String,
}

impl OneShotReturn {
    pub fn new(
        policy_id: &String,
        tokennames: &Vec<String>,
        amounts: &Vec<u64>,
        txhash: &String,
        metadata: &String,
    ) -> Self {
        OneShotReturn {
            policy_id: policy_id.clone(),
            tokennames: tokennames.clone(),
            amounts: amounts.clone(),
            txhash: txhash.clone(),
            metadata: metadata.clone(),
        }
    }
}
