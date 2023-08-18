#![allow(
    dead_code,
    clippy::too_many_arguments,
    unused_assignments,
    unused_variables
)]
// Work in Progress
pub mod ad_params;
pub mod ftairdrop;
pub mod nftairdrop;

pub use crate::error::SleipnirError;
pub use crate::rewards::*;
pub use ad_params::*;

use chrono::{NaiveDateTime, Utc};
use std::str::FromStr;

#[derive(PartialEq, Clone, Debug)]
pub enum ADTokenType {
    FungibleToken,
    NonFungibleToken,
}

impl ToString for ADTokenType {
    fn to_string(&self) -> String {
        match &self {
            ADTokenType::FungibleToken => "FungibleToken".to_string(),
            ADTokenType::NonFungibleToken => "NonFungibleToken".to_string(),
        }
    }
}

impl std::str::FromStr for ADTokenType {
    type Err = SleipnirError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "FungibleToken" => Ok(ADTokenType::FungibleToken),
            "NonFungibleToken" => Ok(ADTokenType::NonFungibleToken),
            _ => Err(SleipnirError::new(&format!(
                "Cannot parse '{src}' into ADTokenType"
            ))),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum ADSelType {
    ScanForHolders,            // Scan for Holder depending on token type
    ScanForHoldersNFTMetaCond, // Scan for NFTs with specific metadata conditions
    //DiscordBotWhitelist, -> Custom
    WalletWhitelist, // -> take an existing wallet whitelist
    //MintingWhitelist, -> Scan for TokenHolders
    Custom,                        // -> Import from csv or sql
    DeligatorsOfStakePoolInEpochX, // Scan pool for addresses in epoch range
    TokenPool,                     // Every Wallet is eligable which did not already claimed
    Combination(Vec<ADSelType>),
}

impl ToString for ADSelType {
    fn to_string(&self) -> String {
        match &self {
            ADSelType::ScanForHolders => "ScanForHolders".to_string(),
            ADSelType::ScanForHoldersNFTMetaCond => "ScanForHoldersNFTMetaCond".to_string(),
            ADSelType::WalletWhitelist => "WalletWhitelist".to_string(),
            ADSelType::Custom => "Custom".to_string(),
            ADSelType::DeligatorsOfStakePoolInEpochX => "DeligatorsOfStakePoolInEpochX".to_string(),
            ADSelType::TokenPool => "TokenPool".to_string(),
            ADSelType::Combination(comb) => {
                let mut ret = String::new();
                for c in comb {
                    ret = ret + &c.to_string() + "|";
                }
                ret.pop();
                ret
            }
        }
    }
}

impl std::str::FromStr for ADSelType {
    type Err = SleipnirError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "ScanForHolders" => Ok(ADSelType::ScanForHolders),
            "ScanForHoldersNFTMetaCond" => Ok(ADSelType::ScanForHoldersNFTMetaCond),
            "WalletWhitelist" => Ok(ADSelType::WalletWhitelist),
            "Custom" => Ok(ADSelType::Custom),
            "DeligatorsOfStakePoolInEpochX" => Ok(ADSelType::DeligatorsOfStakePoolInEpochX),
            "TokenPool" => Ok(ADSelType::TokenPool),
            comb => {
                if !comb.contains('|') {
                    return Err(SleipnirError::new(&format!(
                        "Cannot parse '{src}' into AdSelType"
                    )));
                }
                let csplit: Vec<&str> = comb.split('|').collect();
                let mut combi = Vec::<ADSelType>::new();
                for c in csplit {
                    combi.push(ADSelType::from_str(c)?);
                }

                Ok(ADSelType::Combination(combi))
            }
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum ADDistType {
    StakeDependentOnPools,
    FixedAmoutPerDeligatorOnPools,
    Custom,
    FixedAmoutPerToken,
    FixedAmountDevidedByWallets,
    FixedAmountPerWallet,
    TokenPool,
}

impl ToString for ADDistType {
    fn to_string(&self) -> String {
        match &self {
            ADDistType::StakeDependentOnPools => "StakeDendentOnPools".to_string(),
            ADDistType::FixedAmoutPerDeligatorOnPools => {
                "FixedAmoutPerDeligatorOnPools".to_string()
            }
            ADDistType::Custom => "Custom".to_string(),
            ADDistType::FixedAmoutPerToken => "FixedAmoutPerToken".to_string(),
            ADDistType::FixedAmountDevidedByWallets => "FixedAmountDevidedByWallets".to_string(),
            ADDistType::FixedAmountPerWallet => "FixedAmountPerWallet".to_string(),
            ADDistType::TokenPool => "TokenPool".to_string(),
        }
    }
}

impl std::str::FromStr for ADDistType {
    type Err = SleipnirError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "StakeDependentOnPools" => Ok(ADDistType::StakeDependentOnPools),
            "FixedAmoutPerDeligatorOnPools" => Ok(ADDistType::FixedAmoutPerDeligatorOnPools),
            "Custom" => Ok(ADDistType::Custom),
            "FixedAmoutPerToken" => Ok(ADDistType::FixedAmoutPerToken),
            "FixedAmountDevidedByWallets" => Ok(ADDistType::FixedAmountDevidedByWallets),
            "FixedAmountPerWallet" => Ok(ADDistType::FixedAmountPerWallet),
            "TokenPool" => Ok(ADDistType::TokenPool),
            _ => Err(SleipnirError::new(&format!(
                "Cannot parse '{src}' into AdSelType"
            ))),
        }
    }
}

// Address Lookup Functions for Airdrops
pub async fn create_airdrop(
    network: murin::clib::NetworkIdKind,
    user_id: i64,
    contract_id: Option<i64>,
    airdrop_token_type: ADTokenType,
    airdrop_dist_type: ADDistType,
    airdrop_sel_type: ADSelType,
    vesting_period: Option<String>,
    policy_id: String,
    tokenname: Option<String>,
    pools: Option<Vec<String>>,
    start_epoch: i64,
    end_epoch: Option<i64>,
    ad_dist_params: AirdropDistributionParameter,
    ad_sel_params: AirdropSelectionParameter,
    ad_timing_params: AirdropTimingParameter,
) -> Result<(), SleipnirError> {
    let mut c_id = -1;
    match contract_id {
        Some(id) => {
            c_id = id;
        }
        None => {
            c_id = create_contract(network, user_id, None).await?;
        }
    }

    //create token whitelisting with type airdrop
    let mut vd = chrono::Utc::now();
    if let Some(date) = vesting_period {
        vd = chrono::DateTime::<Utc>::from_utc(
            NaiveDateTime::parse_from_str(&date, "%Y-%m-%d %H:%M:%S")?,
            Utc,
        );
    }

    let mut tn = String::new();
    let mut fingerprint = String::new();
    match airdrop_token_type {
        ADTokenType::FungibleToken => {
            if let Some(name) = tokenname {
                tn = name;
                fingerprint = murin::make_fingerprint(&policy_id.clone(), &tn)?;
            }
        }
        ADTokenType::NonFungibleToken => {
            // We know the Policy ID already, maybe traits in metadata need to be analysed ?
        }
    }

    let mut mconn = mimir::establish_connection()?;
    let current_epoch = mimir::get_epoch(&mut mconn)? as i64;
    if start_epoch < current_epoch {
        return Err(SleipnirError::new(&format!(
            "Start epoch: {start_epoch} cannot be smaller than the current epoch: : {current_epoch:?}"
        )));
    }
    if let Some(endepoch) = end_epoch {
        if endepoch <= current_epoch || endepoch <= start_epoch {
            return Err(SleipnirError::new(&format!(
                "End epoch: {endepoch}, needs to be in future and after start epoch: {start_epoch:?}"
            )));
        }
    }

    //pools is just containing pools if the distribution type is "share depending on stake" or "each deligator of pool" otherwise empty list
    let mut apools = Vec::<gungnir::GPools>::new();
    if airdrop_dist_type == ADDistType::StakeDependentOnPools
        || airdrop_dist_type == ADDistType::FixedAmoutPerDeligatorOnPools
    {
        if let Some(pool) = pools {
            apools.extend(pool.iter().map(|p| gungnir::GPools {
                pool_id: p.clone(),
                first_valid_epoch: current_epoch,
            }));
        }
    }

    // Create Airdrop Parameters
    // equation contains a custom_id to airdrop parameters table -> Juse Database Primary Key 'ID'
    // Airdrop Parameter:
    // - Airdrop Type:          { FT,
    //                            NFT,
    //                          }
    // - Distribution Type(FT): {   share depending on stake,
    //                          fixed amount for each deligator of a pool,
    //                          custom amounts (csv import),
    //                          for each token holder a certain amount,
    //                          fixed amount devided by all receivers
    //                          for each whitelisted address a fixed amount,
    //                          testnet distro for one address
    //                          .....},

    //                           StakeDendentOnPools,
    //                          FixedAmoutPerDeligatorOnPools,
    //                          Custom,
    //                          FixedAmoutPerTokenHolder,
    //                          FixedAmountDevidedByHolders,
    //                          FixedAmountPerAddress,
    //                          TestnetDistro,

    // - Address Selection Type: {  Scan for Holders of NFTs / FTs,
    //                              Discord bot Whitelisting,
    //                              Wallet Whitelisting with message verification,
    //                              Minting - > Mint a Token to whitelist and burn it on claim,
    //                              custom (csv import),
    //                              deligators of a stake pool at a certain epoch,
    //                              testnet distro for one address
    //                              ....
    //                            },
    // - ARGS1:                   { Array of Text, Depending on Distribution Type
    //                              StakeDendentOnPools : [Fixed Amount Of Fungible Tokens to be distributed, Amount Distributed Already, ],
    //                              FixedAmoutPerDeligatorOnPools : [A Fixed Amount Everybody staking with the pool gets, Total Amount, Total Distributed Already],
    //                              Custom: [{JSON Object showing Addresses and amount per Address}],
    //                              FixedAmoutPerTokenHolder: [(The fixed AMount per token),(PolicyID of the Token), (Optional: TokenName of the token), (Min Amount to Hold)],
    //                              FixedAmountDevidedByHolders: [(The fixed amount),(PolicyID od Token),(Optional: TokenName od the TOken), (Min amount to Hold)],
    //                              FixedAmountPerAddress [(The fixed Amount)]
    //                              TestnetDistro: [(stake_addr),(payment_addr),([MintedTokensToReward])]  MintedTokenToReward{(PolicyID,TN,Amount)}
    //                            },
    let args1 = ad_dist_params.to_string_vec();

    // - ARGS2:                   { Array of Text, Depending on Selection Type
    //                              ScanForHoldersFT : [(PolicyID),(TokenName),(fingerprint),(Min Amount to Hold)],
    //                              ScanForHoldersNFT : [(PolicyId)],
    //                              ScanForHoldersNFTMetaCond : [(PolicyId), ([{Metadata-Traits}])],
    //                              DiscordBotWhitelist: [Database Connection Credentials and Tablenames for the Whitelist ; we will nto store to the reward database],
    //                              WalletWhitelisting: [WhitelistingContractId, Max Amount of Whitelistentries],
    //                              MintingWhitelist: [(PolicyId),(Tokenname),(MintingContractId),],
    //                              Custom: (CSV Import as JSON Object see also ADDistType),
    //                              DeligatorsOfStakePoolInEpochX: [PoolId, Epoch],
    //                              Testnet: [(ProvidedStakeAddr)],
    //                              Combination(Vec<ADSelType>): Comes Later,
    //                            },
    let args2 = ad_sel_params.to_string_vec();

    // - ARGS3:                   { Array of Text, Additional Information
    //                                  Repeatable: True|False,
    //                                  Intervall: Weekly (Weekday) | Monthly (Day) | Quarterly(1.x | 15.x) | Each x. of a month,
    //                                  EndDate,
    //                                  StartDate,
    //                            },
    let args3 = ad_timing_params.to_string_vec();

    let mut gconn = gungnir::establish_connection()?;
    let adparam = gungnir::AirDropParameter::create_airdrop_parameter(
        &mut gconn,
        &c_id,
        &user_id,
        &airdrop_token_type.to_string(),
        &airdrop_dist_type.to_string(),
        &airdrop_sel_type.to_string(),
        &args1?,
        &args2?,
        &args3?,
        None,
    )?;

    //start epoch defines when the airdrop can happen / end epoch accordingly restricts the airdrop on epochs

    let _twl = gungnir::TokenWhitelist::create_twl_entry(
        &mut gconn,
        &fingerprint,
        &policy_id,
        &tn,
        &c_id,
        &user_id,
        &vd,
        &apools,
        &gungnir::Calculationmode::AirDrop,
        &adparam.id.to_string(),
        &start_epoch,
        end_epoch.as_ref(),
        None,
    )?;

    // Select whitelisting method for airdrop
    airdrop_whitelist_selection(
        user_id,
        c_id,
        &airdrop_sel_type,
        &adparam,
        &ad_dist_params,
        &ad_sel_params,
    )?;

    // If possible for selected method determine rewards for airdrop
    determine_rewards(user_id, c_id, &airdrop_dist_type, &adparam)?;

    Ok(())
}

pub fn airdrop_whitelist_selection(
    user_id: i64,
    contract_id: i64,
    airdrop_sel_type: &ADSelType,
    adp: &gungnir::AirDropParameter,
    args1: &AirdropDistributionParameter,
    args2: &AirdropSelectionParameter,
) -> Result<(), SleipnirError> {
    match airdrop_sel_type {
        ADSelType::ScanForHolders => {
            match ADTokenType::from_str(&adp.airdrop_token_type)? {
                ADTokenType::FungibleToken => {
                    pub struct ParamScanForHoldersFT {
                        policy_id: murin::clib::PolicyID,
                        tokenname: murin::clib::AssetName,
                        fingerprint: Option<String>,
                        min_holding: Option<i64>,
                    }
                    let adparam = match args2 {
                        AirdropSelectionParameter::ScanForHolders { param } => param,
                        _ => {
                            return Err(SleipnirError::new(
                                "Wrong 'Airdrop Selection Arguments' supplied",
                            ))
                        }
                    };

                    let fingerprint = if let Some(fp) = adparam.fingerprint.clone() {
                        fp
                    } else {
                        murin::make_fingerprint(
                            &hex::encode(adparam.policy_id.to_bytes()),
                            &hex::encode(
                                adparam
                                    .tokenname
                                    .as_ref()
                                    .expect("No TokenName Provided")
                                    .to_bytes(),
                            ),
                        )?
                    };

                    let whitelist =
                        mimir::lookup_token_holders(&fingerprint, adparam.min_holding.as_ref())?;
                }
                ADTokenType::NonFungibleToken => {
                    let adparam = match args2 {
                        AirdropSelectionParameter::ScanForHoldersNFT { param } => param,
                        _ => {
                            return Err(SleipnirError::new(
                                "Wrong 'Airdrop Selection Arguments' supplied",
                            ))
                        }
                    };

                    // ToDo: Traits

                    let whitelist = mimir::lookup_nft_token_holders(&hex::encode(
                        adparam.policy_id.to_bytes(),
                    ))?;
                }
            }
        }
        ADSelType::ScanForHoldersNFTMetaCond => {
            // Check for all "latests" mint transactions which contain a token with the given policy ID
            // where the metadata of the minting transaction contain the given trait
            // Return all matching NFTs (Tokens)

            // Second Step
            // Lookup current holders of those Tokens and return stake addresses
        }
        //ADSelType::MintingWhitelist => {
        // Is same option as scan for Tokens (NFT or Token) but needs to pmint and distribute those first
        //},
        //ADSelType::DiscordBotWhitelist => {
        // Is Custom Import from .csv or sql
        //},
        ADSelType::DeligatorsOfStakePoolInEpochX => {
            // Scan all wallets of a specif epoch range and add to whitelist
        }
        ADSelType::WalletWhitelist => {
            // Select an already created whitelist id
        }
        ADSelType::Custom => {
            // import csv file
        }
        ADSelType::TokenPool => {
            // On A Tokenpool there is no whitelist, each wallet is whitelisted, by doing a claim the wallet gets added to
            // "blacklist" which is just for this specific contract and forbidds further claims.
            // TokenPool airdrops are blockchain wide airdrops and can just be handleded for themself and not claim tokens with
            // usual rewards together (depends maybe we can blacklist on token_whitelist level for a contract)
        }

        ADSelType::Combination(combo) => {
            // Later or not needed
        }
    }
    Ok(())
}

/// Creates the rewards for a given whitelist
pub fn determine_rewards(
    user_id: i64,
    contract_id: i64,
    airdrop_dist_type: &ADDistType,
    adp: &gungnir::AirDropParameter,
    //whitelist : Option<&Vec::<gungnir::WhitelistedWallet>>,
) -> Result<(), SleipnirError> {
    match airdrop_dist_type {
        ADDistType::FixedAmountDevidedByWallets => {
            // a fixed amount of total rewards is devided between all wallets
        }
        ADDistType::FixedAmountPerWallet => {
            // each wallet in the whitelist gets a fixed amount of rewards
        }
        ADDistType::FixedAmoutPerDeligatorOnPools => {
            // For each delegiator on a set of pools (optional: above a certain stake limit)
            // the wallets get a fixed amount of rewards
        }
        ADDistType::FixedAmoutPerToken => {
            // For each Token the wallet is holding it gets a certain amount
        }
        ADDistType::StakeDependentOnPools => {
            // Dependent on the Ada Stake in specific pools the stake amount is multiplied with a factor,
            // this specifies the reward for each wallet
        }
        ADDistType::TokenPool => {
            // On TOken Pools we distribute a fixed amount once to each wallet for a whitelisted token,
            // The TokenPool needs a special contract as the claim and the reward are created at the same time.
            // Is it possible to just create claim? As we need a special contract type we can check for existing claims
            // on transaction creation
            // THe whole token pool system should be limited per day / epoch to make the system "fairer" after having a limited amount paid per / epoch / day
            // the same amount is available next epoch / day again
        }
        ADDistType::Custom => {
            // Add csvfile same csv file as for token selection used
            //
        }
    }
    Ok(())
}
