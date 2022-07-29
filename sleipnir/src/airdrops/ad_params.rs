/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::error::SleipnirError;







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

    pub struct ParamFTStakeDependentDiv {
        // Amount to be distributed to all deligators, largest deligator gets most tokens
        distribution_amount : i64,
        token_policy : murin::clib::PolicyID,
        token_name : murin::clib::AssetName,
        token_fingerprint : String, 
        // Just consider deligators over min_stake
        min_stake : Option<i64>,
    }

    pub struct ParamNFTStakeDependent {
        min_stake : i64,
        token_policy : murin::clib::PolicyID,
        // Multiple shal allow to distribute more than one NFT to a person which is staking a factor of "mutli_factor"
        multiple : Option<bool>, 
        // the amount needed to be staked on top of "min_stake" to generate the multiple_factor 
        multi_factor : Option<i64>,
        // maximum NFTs to be provided 
        multi_max : Option<i8>,
    }
    
    pub struct ParamFTStakeDependentFix {
        min_stake : i64,
        token_policy : murin::clib::PolicyID,
        token_name : murin::clib::AssetName,
        token_fingerprint : String, 
        // Amount to be distributed for each deligator above min_stake
        distribution_amount : i64,
    }

    // Distribute depending on holded tokens, when combined with a whitelist just the whitelist entries will generate rewards
    // If no whitelist is connected it will search for all holders 
    pub struct ParamHolderAmountPerToken{
        min_holding_token : i64,
        holding_token_policy : murin::clib::PolicyID,
        holding_token_name : Option<murin::clib::AssetName>,
        holding_token_fingerprint : Option<String>,
        // Amount to be distributed for each token above min_holding_token
        distribution_amount : i64,
        // If devide = true then devide the distribution amount between all holders above min_holding_token depending on the amount of tokens they hold
        devide : bool,
        dist_token_policy : murin::clib::PolicyID,
        dist_token_name : Option<murin::clib::AssetName>,
        dist_token_fingerprint : Option<String>,        
    }

    pub struct ParamFixedperAddress{
        distribution_amount : i64,
        dist_token_policy : murin::clib::PolicyID,
        dist_token_name : Option<murin::clib::AssetName>,
        dist_token_fingerprint : Option<String>,
    }

    // ToDo: 
    // Is based on a csv import, table needs to be imported, rewards are generated on the fly; Possibility to activate / deactivate rewards ? 
    pub struct ParamCustom{
        table : Vec<(murin::clib::address::Address,i64,Option<murin::clib::AssetName>)>,
        dist_token_policy : murin::clib::PolicyID,
        dist_token_name : Option<murin::clib::AssetName>,
        dist_token_fingerprint : Option<String>,
    }

    pub enum AirdropDistributionParameter {
        FTStakeDependentDiv{param : ParamFTStakeDependentDiv},
        FTStakeDependentFix{param : ParamFTStakeDependentFix},
        NFTStakeDependent{param: ParamNFTStakeDependent},
        HolderAmountPerToken{param : ParamHolderAmountPerToken},
        FixedperAddress{param : ParamFixedperAddress},
        Custom{param : ParamCustom}
    }

    impl AirdropDistributionParameter {
        pub fn to_string_vec(&self) -> Result<Vec::<String>,SleipnirError> {
            let mut out = Vec::<String>::new();
            out.push("Not Implemented".to_string());
            Ok(out)
        }
    }


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


    pub struct MetadataTraits {
        traits          : Vec::<String>, 
    }

    pub struct ParamScanForHoldersNFT {
        pub policy_id   : murin::clib::PolicyID,
        traits          : Option<MetadataTraits>, 
    }

    pub struct ParamScanForHolders {
        pub policy_id   : murin::clib::PolicyID,
        pub tokenname   : Option<murin::clib::AssetName>,
        pub fingerprint : Option<String>, 
        pub min_holding : Option<i64>,
    }

    

    pub struct ParamExistingWhitelist {
        whitelist_id : i64,
    }

    // The difference to "ParamExistingWhitelist" is that the list is created in a previous step 
    pub struct ParamImportWalletList {
        whitelist_id : i64,
    }

    // Make sure that stake_addresses are not considered twice over epochs / pools
    // Think about how a pool can approve an Airdrop
    pub struct ParamDeligatorsInEpoch {
        pool_ids : Vec<String>,
        epochs :  Vec<i64>,
    }

    pub enum AirdropSelectionParameter {
        ScanForHoldersNFT{param : ParamScanForHoldersNFT},
        ScanForHolders{param: ParamScanForHolders},
        ExistingWhitelist{param : ParamExistingWhitelist},
        ImportWalletList{param : ParamImportWalletList},
        DeligatorsInEpoch{param : ParamDeligatorsInEpoch},
        TokenPool,
        None,
    } 

    impl AirdropSelectionParameter {
        pub fn to_string_vec(&self) -> Result<Vec::<String>,SleipnirError> {
            let mut out = Vec::<String>::new();
            out.push("Not Implemented".to_string());
            Ok(out)
        }
    }



    // - ARGS3:                   { Array of Text, Additional Information
    //                                  Repeatable: True|False, 
    //                                  Intervall: Weekly (Weekday) | Monthly (Day) | Quarterly(1.x | 15.x) | Each x. of a month, 
    //                                  EndDate, 
    //                                  StartDate, 
    //                            },


pub struct ParamWeekly {
    weekday : chrono::Weekday,
    time    : chrono::NaiveTime,
}

pub struct ParamMonthly {
    // Limit Days 1 to 20 or similar
    day     : i8,
    // Time at the date the reward becomes available
    time    : chrono::NaiveTime,
}

pub struct ParamQuarterly {
    // Limit Days 1 to 20 or similar
    // Day of the first month of a quarter
    day : i8
}

pub enum AirdropInterval {
    Weekly{param : ParamWeekly},
    Monthly{param : ParamMonthly},
    Quarterly{param : ParamQuarterly},
    // Triggered means the user can click a button in the admin panel to create the exact same rewards again
    Triggered
}

pub struct AirdropTimingParameter {
    repeatable : bool,
    interval : Option<AirdropInterval>,
    start_date : chrono::DateTime<chrono::Utc>,
    end_date : Option<chrono::DateTime<chrono::Utc>>,

}

impl AirdropTimingParameter {
    pub fn to_string_vec(&self) -> Result<Vec::<String>,SleipnirError> {
        let mut out = Vec::<String>::new();
        out.push("Not Implemented".to_string());
        Ok(out)
    }
}