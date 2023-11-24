use std::collections::HashMap;
use std::io::Error;
use std::str::FromStr;

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, Utc};
use drasil_gungnir::{Rewards, TokenInfo};
use drasil_murin::address::Address;
use drasil_murin::cardano;
use drasil_murin::error::MurinError;
use drasil_murin::stdtx::{AssetTransfer, StdAssetHandle};
use drasil_murin::utils::to_bignum;
use drasil_murin::wallet;
use drasil_murin::worldmobile::wmtstaking::{StakeTxData, UnStakeTxData};
use drasil_murin::{AssetName, PolicyID, TxData};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIs, EnumString, EnumVariantNames};

use crate::protocol::worldmobile::staking::StakingAction;

#[derive(
    Serialize, Deserialize, Debug, Clone, Eq, PartialEq, EnumVariantNames, Display, EnumString,
)]
pub enum ContractType {
    MarketPlace,
    WmtStaking,
    DrasilAPILiquidity,
}

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, EnumVariantNames, Display, EnumString,
)]
pub enum MarketplaceActions {
    List,
    Buy,
    Cancel,
    Update,
}

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, EnumVariantNames, Display, EnumString,
)]
pub enum MultiSigType {
    SpoRewardClaim,
    Mint,
    NftCollectionMinter,
    ClAPIOneShotMint,
    TestRewards,
    UTxOpti,
    Other,
    CustomerPayout,
}

pub struct Utxopti {}
impl FromStr for Utxopti {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "utxoopti" => Ok(Utxopti {}),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Wrong Type".to_string(),
            )),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Display, EnumString, EnumVariantNames)]
pub enum StdTxType {
    DelegateStake,
    StandardTx,
    DeregisterStake,
    RewardWithdrawal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signature {
    signature: String,
}

impl Signature {
    pub fn get_signature(&self) -> String {
        self.signature.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ContractAction {
    MarketplaceActions(MarketplaceActions),
    StakingAction(StakingAction),
}

impl ContractAction {}

impl FromStr for ContractAction {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let action = match src.to_lowercase().as_str() {
            "list" => ContractAction::MarketplaceActions(MarketplaceActions::List),
            "buy" => ContractAction::MarketplaceActions(MarketplaceActions::Buy),
            "cancel" => ContractAction::MarketplaceActions(MarketplaceActions::Cancel),
            "update" => ContractAction::MarketplaceActions(MarketplaceActions::Update),
            "stake" => ContractAction::StakingAction(StakingAction::Stake),
            "unstake" => ContractAction::StakingAction(StakingAction::UnStake),
            _ => {
                return Err(Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("ContractAction '{src}' does not exist"),
                ))
            }
        };
        Ok(action)
    }
}

impl ToString for ContractAction {
    fn to_string(&self) -> String {
        match &self {
            ContractAction::MarketplaceActions(action) => action.to_string().to_lowercase(),
            ContractAction::StakingAction(action) => action.to_string().to_lowercase(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinterToken {
    tokenname: String,
    amount: u64,
}

impl MinterToken {
    pub fn into_mintasset(
        &self,
    ) -> Result<drasil_murin::txbuilder::MintTokenAsset, drasil_murin::MurinError> {
        let tn = cardano::string_to_assetname(&self.tokenname)?;
        let amt = cardano::u64_to_bignum(self.amount);
        Ok((None, tn, amt))
    }

    pub fn for_all_into_mintasset(
        ut: &Vec<MinterToken>,
    ) -> Result<Vec<drasil_murin::txbuilder::MintTokenAsset>, drasil_murin::MurinError> {
        let mut out = Vec::<drasil_murin::txbuilder::MintTokenAsset>::new();
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
    pub fn into_asset(
        &self,
    ) -> Result<drasil_murin::txbuilder::TokenAsset, drasil_murin::MurinError> {
        let cs = drasil_murin::cardano::string_to_policy(&self.currencysymbol)?;
        let tn = drasil_murin::cardano::string_to_assetname(&self.tokenname)?;
        let amt = drasil_murin::cardano::u64_to_bignum(self.amount);
        Ok((cs, tn, amt))
    }

    pub fn for_all_into_asset(
        ut: &Vec<Token>,
    ) -> Result<Vec<drasil_murin::txbuilder::TokenAsset>, drasil_murin::MurinError> {
        let mut out = Vec::<drasil_murin::txbuilder::TokenAsset>::new();
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

#[derive(Serialize, Deserialize, Debug, Clone, Display, EnumString, EnumVariantNames)]
pub enum Network {
    Testnet,
    Mainnet,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TXPWrapper {
    TransactionPattern(Box<TransactionPattern>),
    Signature(Signature),
    OneShotMinter(OneShotMintPayload),
    WalletTransaction(),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletTransactionPattern {
    user: Option<String>,
    contract_id: Option<u64>, // ToDO: Expect a Vector instead of a single contract; needs to be changed on front-end
    wallet_type: Option<WalletType>, // yoroi, ccvault, gero, flint, ... // or yoroi, cip30, typhon
    #[serde(alias = "sending_wal_addrs")]
    used_addresses: Option<Vec<String>>,
    unused_addresses: Option<Vec<String>>,
    #[serde(alias = "sending_stake_addr")]
    stake_address: Option<String>,
    change_address: Option<String>,
    #[serde(alias = "inputs")]
    utxos: Option<Vec<String>>,
    excludes: Option<Vec<String>>,
    collateral: Option<Vec<String>>,
    network: Option<u64>,
    #[serde(alias = "script")]
    operation: Operation,
}

impl WalletTransactionPattern {
    pub fn into_txp(&self) -> TransactionPattern {
        TransactionPattern {
            user: None,
            contract_id: None,
            wallet_type: None,
            used_addresses: Vec::<String>::new(),
            stake_address: None,
            change_address: None,
            utxos: Some(Vec::<String>::new()),
            excludes: None,
            collateral: None,
            operation: self.operation.clone(),
            network: 0,
            unused_addresses: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionPattern {
    user: Option<String>,
    contract_id: Option<u64>, // ToDO: Expect a Vector instead of a single contract; needs to be changed on front-end
    wallet_type: Option<WalletType>, // yoroi, ccvault, gero, flint, ... // or yoroi, cip30, typhon
    #[serde(alias = "sending_wal_addrs")]
    used_addresses: Vec<String>,
    unused_addresses: Option<Vec<String>>,
    #[serde(alias = "sending_stake_addr")]
    stake_address: Option<String>,
    change_address: Option<String>,
    #[serde(alias = "inputs")]
    utxos: Option<Vec<String>>,
    excludes: Option<Vec<String>>,
    collateral: Option<Vec<String>>,
    #[serde(alias = "script")]
    operation: Operation,
    network: u64,
}

impl TransactionPattern {
    pub fn new_empty(customer_id: u64, script_spec: &Operation, network: u64) -> Self {
        TransactionPattern {
            user: Some(customer_id.to_string()),
            contract_id: None,
            wallet_type: None,
            used_addresses: Vec::<String>::new(),
            stake_address: None,
            change_address: None,
            utxos: Some(Vec::<String>::new()),
            excludes: None,
            collateral: None,
            operation: script_spec.clone(),
            network,
            unused_addresses: None,
        }
    }

    pub fn user(&self) -> String {
        if let Some(u) = self.user.clone() {
            u
        } else {
            "".to_string()
        }
    }

    pub fn contract_id(&self) -> Option<u64> {
        // ToDO: Expect a Vector instead of a single contract; needs to be changed on front-end
        self.contract_id
    }

    pub fn wallet_type(&self) -> Option<WalletType> {
        self.wallet_type.clone()
    }

    pub fn used_addresses(&self) -> Vec<String> {
        self.used_addresses.clone()
    }

    pub fn set_used_addresses(&mut self, vec: &[String]) {
        self.used_addresses = vec.to_owned();
    }

    pub fn set_contract_id(&mut self, n: u64) {
        self.contract_id = Some(n);
    }

    pub fn stake_addr(&self) -> Option<String> {
        self.stake_address.clone()
    }

    pub fn utxos(&self) -> Option<Vec<String>> {
        self.utxos.clone()
    }

    pub fn excludes(&self) -> Option<Vec<String>> {
        self.excludes.clone()
    }

    pub fn collateral(&self) -> Option<String> {
        match &self.collateral {
            Some(col) => {
                if !col.is_empty() {
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

    pub fn operation(&self) -> Option<Operation> {
        Some(self.operation.clone())
    }

    pub async fn into_txdata(
        &self,
    ) -> Result<drasil_murin::txbuilder::TxData, drasil_murin::error::MurinError> {
        let inputs = match self.utxos() {
            None => {
                return Err(drasil_murin::error::MurinError::new(
                    "Cannot build transaction data, no inputs provided",
                ))
            }
            Some(data) => data,
        };

        let saddr = match self.stake_addr() {
            Some(sa) => wallet::decode_address_from_bytes(&sa).await.ok(),
            None => None,
        };

        let contract_id = match self.contract_id() {
            Some(n) => n as i64,
            None => -1,
        };
        let mut txd = TxData::new(
            Some(vec![contract_id]), // ToDO: Expect a Vector instead of a single contract; needs to be changed on front-end
            wallet::addresses_from_string(&self.used_addresses()).await?,
            saddr,
            wallet::transaction_unspent_outputs_from_string_vec(
                inputs.as_ref(),
                self.collateral().as_ref().map(|x| vec![x.clone()]).as_ref(),
                self.excludes().as_ref(),
            )?,
            wallet::get_network_kind(self.network).await?,
            0u64,
        )?;

        if let Some(collateral) = self.collateral() {
            txd.set_collateral(wallet::transaction_unspent_outputs_from_string(
                &collateral,
            )?)
        }

        if let Some(excludes) = self.excludes() {
            txd.set_excludes(wallet::transaction_unspent_outputs_from_string_vec(
                &excludes, None, None,
            )?)
        }

        Ok(txd)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumIs)]
pub enum Operation {
    SpoRewardClaim {
        rewards: Vec<drasil_murin::RewardHandle>,
        recipient_stake_addr: String,
        recipient_payment_addr: String,
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
    Minter {
        mint_tokens: Option<Vec<MinterToken>>,
        receiver_stake_addr: Option<String>,
        receiver_payment_addr: String,
        mint_metadata: Option<String>,
        auto_mint: Option<bool>,
        contract_id: i64,
    },
    NftCollectionMinter {
        mint_handles: Vec<MintRewardHandle>,
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
        addresses: Option<Vec<String>>,
    },
    StakeDeregistration {
        payment_addresses: Option<Vec<String>>,
    },
    RewardWithdrawal {
        withdrawal_amount: u64,
    },
    StdTx {
        transfers: Vec<TransferHandle>,
        wallet_addresses: Option<Vec<String>>,
    },
    CPO {
        po_id: i64,
        pw: String,
    },
    ClApiOneShotMint {
        tokennames: Vec<String>,
        amounts: Vec<u64>,
        metadata: drasil_murin::minter::Cip25Metadata,
        receiver: String,
    },
    WmtStaking {
        /// The EN to stake to.
        ennft: String,
        /// The staking amount.
        amount: u64,
    },
}

impl Operation {
    pub async fn into_mp(
        &self,
        avail_inputs: drasil_murin::TransactionUnspentOutputs,
    ) -> Result<drasil_murin::txbuilder::marketplace::MpTxData, drasil_murin::error::MurinError>
    {
        use drasil_murin::txbuilder::marketplace::MpTxData;

        match self {
            Operation::Marketplace {
                tokens,
                metadata,
                royalties_addr,
                royalties_rate,
                selling_price,
            } => {
                let assets = Token::for_all_into_asset(tokens)?;
                let token_utxos = drasil_murin::txbuilder::find_token_utxos_na(
                    &avail_inputs,
                    assets.clone(),
                    None,
                )?;

                let mut mptx = MpTxData::new(assets, token_utxos, *selling_price);

                if let Some(royaddr) = royalties_addr {
                    mptx.set_royalties_address(wallet::decode_address_from_bytes(royaddr).await?);
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
                "provided wrong specfic parameter for this contract",
            )),
        }
    }

    pub async fn into_rwd(
        &self,
    ) -> Result<drasil_murin::txbuilder::rwdist::RWDTxData, drasil_murin::error::MurinError> {
        use drasil_murin::txbuilder::rwdist::RWDTxData;

        match self {
            Operation::SpoRewardClaim {
                rewards,
                recipient_stake_addr,
                recipient_payment_addr,
            } => {
                // let assets = Token::for_all_into_asset(reward_tokens)?;
                let stake_addr = wallet::decode_address_from_bytes(recipient_stake_addr).await?;
                let payment_addr =
                    wallet::decode_address_from_bytes(recipient_payment_addr).await?;

                Ok(RWDTxData::new(rewards, &stake_addr, &payment_addr))
            }
            _ => Err(MurinError::new(
                "provided wrong specfic parameter for this contract",
            )),
        }
    }

    pub async fn into_stdassettx(
        &self,
    ) -> Result<drasil_murin::txbuilder::stdtx::StandardTxData, drasil_murin::error::MurinError>
    {
        use drasil_murin::txbuilder::stdtx::StandardTxData;

        match self {
            Operation::StdTx {
                transfers,
                wallet_addresses,
            } => {
                let mut trans = Vec::<AssetTransfer>::new();
                for t in transfers {
                    let receiver =
                        wallet::address_from_string_non_async(&t.receiving_address).unwrap();
                    let mut assets = Vec::<StdAssetHandle>::new();
                    let metadata = t.message.clone();
                    for n in &t.asset_handles {
                        let policy = if let Some(p) = &n.policy {
                            Some(PolicyID::from_hex(p)?)
                        } else {
                            None
                        };
                        let tokenname = if let Some(tn) = &n.tokenname {
                            Some(AssetName::new(hex::decode(tn)?)?)
                        } else {
                            None
                        };

                        assets.push(StdAssetHandle {
                            fingerprint: n.fingerprint.clone().as_ref().map(|f| f.to_string()),
                            policy,
                            tokenname,
                            amount: to_bignum(n.amount),
                            metadata: n.metadata.as_ref().cloned(),
                        })
                    }
                    trans.push(AssetTransfer {
                        receiver,
                        assets,
                        metadata,
                    })
                }
                let wal_addr = if let Some(addr) = wallet_addresses {
                    let r = addr.iter().fold(Vec::<Address>::new(), |mut acc, a| {
                        acc.push(wallet::address_from_string_non_async(a).unwrap());
                        acc
                    });
                    r
                } else {
                    vec![]
                };

                Ok(StandardTxData {
                    wallet_addresses: wal_addr,
                    transfers: trans,
                })
            }
            _ => Err(MurinError::new(
                "provided wrong specfic parameters for the transaction type",
            )),
        }
    }

    pub async fn into_colmintdata(
        &self,
    ) -> Result<
        drasil_murin::txbuilder::minter::models::ColMinterTxData,
        drasil_murin::error::MurinError,
    > {
        use drasil_murin::txbuilder::minter::models::*;

        match self {
            Operation::NftCollectionMinter { mint_handles } => {
                let mut out = Vec::<CMintHandle>::new();
                for handle in mint_handles {
                    let mrwd = drasil_gungnir::minting::models::MintReward::get_mintreward_by_id(
                        handle.id,
                    )
                    .unwrap();
                    if mrwd.pay_addr != handle.addr {
                        return Err(MurinError::new("corrupt data"));
                    }
                    out.push(CMintHandle {
                        id: mrwd.id,
                        project_id: mrwd.project_id,
                        pay_addr: mrwd.pay_addr,
                        nft_ids: mrwd.nft_ids.iter().map(hex::encode).collect(),
                        v_nfts_b: mrwd.v_nfts_b.iter().map(hex::encode).collect(),
                    })
                }
                Ok(ColMinterTxData::new(out))
            }
            _ => Err(MurinError::new(
                "provided wrong specfic parameter for this contract",
            )),
        }
    }

    pub async fn into_mintdata(
        &self,
    ) -> Result<drasil_murin::txbuilder::minter::MinterTxData, drasil_murin::error::MurinError>
    {
        use drasil_murin::txbuilder::minter::MinterTxData;

        match self {
            Operation::Minter {
                mint_tokens,
                receiver_stake_addr,
                receiver_payment_addr,
                mint_metadata,
                auto_mint,
                contract_id,
            } => {
                let assets = match mint_tokens {
                    Some(tokens) => MinterToken::for_all_into_mintasset(tokens)?,
                    None => Vec::<drasil_murin::MintTokenAsset>::new(),
                };
                let stake_addr = match receiver_stake_addr {
                    Some(addr) => Some(wallet::decode_address_from_bytes(addr).await?),
                    None => None,
                };
                let payment_addr = wallet::decode_address_from_bytes(receiver_payment_addr).await?;
                let metadata = match mint_metadata {
                    Some(data) => {
                        if !data.is_empty() {
                            log::debug!("Serde deserializing script parameter");
                            serde_json::from_str(data)?
                        } else {
                            drasil_murin::minter::Cip25Metadata::new()
                        }
                    }
                    None => drasil_murin::minter::Cip25Metadata::new(),
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
            Operation::ClApiOneShotMint {
                tokennames,
                amounts,
                metadata,
                receiver,
            } => {
                log::debug!("Try to parse OneShotType");
                let mut assets = Vec::<drasil_murin::txbuilder::MintTokenAsset>::new();
                for (i, t) in tokennames.iter().enumerate() {
                    let tn =
                        drasil_murin::cardano::string_to_assetname(&hex::encode(t.as_bytes()))?;
                    let amt = drasil_murin::cardano::u64_to_bignum(amounts[i]);
                    assets.push((None, tn, amt))
                }
                let payment_addr = wallet::address_from_string(receiver).await?;
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
                "provided wrong specfic parameter for this contract",
            )),
        }
    }

    pub async fn into_stake_delegation(
        &self,
    ) -> Result<drasil_murin::txbuilder::stdtx::DelegTxData, MurinError> {
        use drasil_murin::txbuilder::stdtx::DelegTxData;

        match self {
            Operation::StakeDelegation {
                poolhash,
                addresses: _,
            } => Ok(DelegTxData::new(poolhash)?),
            _ => Err(MurinError::new(
                "provided wrong specfic parameter for this transaction",
            )),
        }
    }

    pub async fn into_withdrawal(
        &self,
    ) -> Result<drasil_murin::txbuilder::stdtx::WithdrawalTxData, drasil_murin::error::MurinError>
    {
        use drasil_murin::txbuilder::stdtx::WithdrawalTxData;
        match self {
            Operation::RewardWithdrawal { withdrawal_amount } => {
                Ok(WithdrawalTxData::new(*withdrawal_amount)?)
            }
            _ => Err(MurinError::new(
                "provided wrong specific parameter for this withdrawal transaction",
            )),
        }
    }

    pub async fn into_stake_deregistration(
        &self,
    ) -> Result<drasil_murin::txbuilder::stdtx::DeregTxData, drasil_murin::error::MurinError> {
        use drasil_murin::txbuilder::stdtx::DeregTxData;
        match self {
            Operation::StakeDeregistration { .. } => Ok(DeregTxData::new()?),
            _ => Err(MurinError::new(
                "provided wrong specfic parameter for this transaction",
            )),
        }
    }

    pub async fn into_cpo(
        &self,
    ) -> Result<drasil_murin::txbuilder::CPO, drasil_murin::error::MurinError> {
        use drasil_murin::txbuilder::CPO;

        match self {
            Operation::CPO { po_id, pw } => Ok(CPO::new(*po_id, pw.to_owned())),

            _ => Err(MurinError::new(
                "provided wrong specfic parameter for this transaction",
            )),
        }
    }

    pub async fn into_wmt_staking(&self) -> Result<StakeTxData, MurinError> {
        match self {
            Operation::WmtStaking { amount, ennft } => {
                Ok(StakeTxData::new(*amount, ennft.to_owned()))
            }
            _ => Err(MurinError::new(
                "provided wrong specfic parameter for wmt staking transaction",
            )),
        }
    }

    pub async fn into_wmt_unstaking(&self) -> Result<UnStakeTxData, MurinError> {
        match self {
            Operation::WmtStaking { ennft, amount } => {
                let unstake_data = UnStakeTxData::new(ennft.to_owned(), *amount);
                Ok(unstake_data)
            }
            _ => Err(MurinError::new(
                "provided wrong specfic parameter for wmt staking transaction",
            )),
        }
    }
}

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Display, EnumString, EnumVariantNames,
)]
pub enum WalletType {
    #[serde(alias = "Nami", rename(deserialize = "nami"))]
    Nami,
    #[serde(alias = "ccvault", rename(deserialize = "eternl"))]
    Eternl,
    #[serde(alias = "Gero", rename(deserialize = "gero"))]
    Gero,
    #[serde(alias = "Flint", rename(deserialize = "flint"))]
    Flint,
    #[serde(alias = "Yoroi", rename(deserialize = "yoroi"))]
    Yoroi,
    #[serde(alias = "Typhon", rename(deserialize = "typhon"))]
    Typhon,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReturnError {
    pub msg: String,
}

impl ReturnError {
    pub fn new(str: &str) -> ReturnError {
        ReturnError {
            msg: str.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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

    pub fn set_tx(&mut self, s: &String) {
        self.tx = s.to_string();
    }

    pub fn set_id(&mut self, s: &String) {
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
        let slice: Vec<&str> = src.split('|').collect();
        if slice.len() != 2 {
            Err(Error::new(std::io::ErrorKind::InvalidData, src.to_string()))
        } else {
            Ok(UnsignedTransaction {
                id: slice[0].to_string(),
                tx: slice[1].to_string(),
            })
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TxHash {
    txhash: String,
}

impl TxHash {
    pub fn new(str: &String) -> TxHash {
        TxHash {
            txhash: str.to_string(),
        }
    }

    pub fn set_txhash(&mut self, str: &String) {
        self.txhash = str.to_string();
    }
}

// Client API Types
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OneShotMintPayload {
    tokennames: Vec<String>,
    amounts: Vec<u64>,
    metadata: drasil_murin::minter::Cip25Metadata,
    receiver: String,
    network: u8,
}

impl OneShotMintPayload {
    pub fn into_script_spec(&self) -> Operation {
        Operation::ClApiOneShotMint {
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

    pub fn metadata(&self) -> drasil_murin::minter::Cip25Metadata {
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
        policy_id: &str,
        tokennames: &[String],
        amounts: &[u64],
        txhash: &str,
        metadata: &str,
    ) -> Self {
        OneShotReturn {
            policy_id: policy_id.to_owned(),
            tokennames: tokennames.to_owned(),
            amounts: amounts.to_owned(),
            txhash: txhash.to_owned(),
            metadata: metadata.to_owned(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClaimedHandle {
    pub stake_addr: String,
    pub payment_addr: String,
    pub policyid: String,
    pub tokenname: String,
    pub fingerprint: String,
    pub amount: BigDecimal,
    pub contract_id: i64,
    pub user_id: i64,
    pub txhash: String,
    pub invalid: Option<bool>,
    pub invalid_descr: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ClaimedHandle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stake_addr: String,
        payment_addr: String,
        policyid: String,
        tokenname: String,
        fingerprint: String,
        amount: BigDecimal,
        contract_id: i64,
        user_id: i64,
        txhash: String,
        invalid: Option<bool>,
        invalid_descr: Option<String>,
        timestamp: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> ClaimedHandle {
        ClaimedHandle {
            stake_addr,
            payment_addr,
            policyid,
            tokenname,
            fingerprint,
            amount,
            contract_id,
            user_id,
            txhash,
            invalid,
            invalid_descr,
            timestamp,
            updated_at,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RewardHandle {
    pub contract_id: i64,
    pub stake_addr: String,
    pub fingerprint: String,
    pub policy: String,
    pub tokenname: String,
    pub tot_earned: i128,
    pub tot_claimed: i128,
    pub last_calc_epoch: i64,
}

impl RewardHandle {
    pub fn new(ti: &TokenInfo, rwd: &Rewards) -> RewardHandle {
        RewardHandle {
            contract_id: rwd.contract_id,
            stake_addr: rwd.stake_addr.clone(),
            fingerprint: ti.fingerprint.clone().unwrap(),
            policy: ti.policy.clone(),
            tokenname: ti.tokenname.clone().unwrap(),
            tot_earned: (rwd.tot_earned.clone() / &BigDecimal::from_i32(1000000).unwrap())
                .to_i128()
                .unwrap(),

            tot_claimed: rwd.tot_claimed.clone().to_i128().unwrap(),
            last_calc_epoch: rwd.last_calc_epoch,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MintRewardHandle {
    pub id: i64,
    pub addr: String,
    pub nfts: Option<Vec<String>>,
    pub project: MintProjectHandle,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MintProjectHandle {
    pub project_name: String,
    pub collection_name: String,
    pub author: String,
    pub image: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct AssetHandle {
    pub fingerprint: Option<String>,
    pub policy: Option<String>,
    pub tokenname: Option<String>,
    pub amount: u64,
    pub metadata: Option<serde_json::Value>,
}

impl AssetHandle {
    pub fn same_asset(&self, other: &Self) -> bool {
        match self.policy {
            Some(_) => {
                self.fingerprint == other.fingerprint
                    && self.policy == other.policy
                    && self.tokenname == other.tokenname
            }
            None => {
                other.policy.is_none()
                    && other.fingerprint.is_none()
                    && other.tokenname.is_none()
                    && self.tokenname.is_none()
                    && self.fingerprint.is_none()
            }
        }
    }

    pub fn new_empty() -> Self {
        AssetHandle {
            fingerprint: None,
            policy: None,
            tokenname: None,
            amount: 0,
            metadata: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TransferHandle {
    pub receiving_address: String,
    pub asset_handles: Vec<AssetHandle>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use drasil_murin::clib::crypto::Ed25519KeyHash;
    use tokio;
    #[tokio::test]
    async fn stake_deregistration() {
        let _poolhash = "pool1a7h89sr6ymj9g2a9tm6e6dddghl64tp39pj78f6cah5ewgd4px0".to_string();
        let addr1 = "stake_test1uqd2nz8ugrn6kwkflvmt9he8dr966dszfmm5lt66qdmn28qt4wff9";
        let payment_addresses = Some(vec![addr1.to_string()]);
        let op = super::Operation::StakeDeregistration { payment_addresses };

        let deregistration = op.into_stake_deregistration().await.unwrap();

        assert!(!deregistration.get_registered()); // registration status undefined => null => false
    }

    #[tokio::test]
    async fn stake_delegation() {
        let poolhash = "pool1pt39c4va0aljcgn4jqru0jhtws9q5wj8u0xnajtkgk9g7lxlk2t".to_string();
        let addr1 = "stake_test1uqnfwu6xlrp95yhkzq0q5p3ct2adrrt92vx5yqsr4ptqkugn5s708".to_string();
        let addresses = Some(vec![addr1]);
        let op = super::Operation::StakeDelegation {
            poolhash: poolhash.clone(),
            addresses,
        };

        let delegation = op.into_stake_delegation().await.unwrap();

        let real_poolkeyhash = Ed25519KeyHash::from_bech32(&poolhash).unwrap();
        let real_registered = false; // Placeholder value. Unit test can't test this. Needs integration test.
        assert_eq!(delegation.get_poolhash(), poolhash);
        assert_eq!(delegation.get_poolkeyhash(), real_poolkeyhash);
        assert_eq!(delegation.get_registered(), real_registered);
    }
}
