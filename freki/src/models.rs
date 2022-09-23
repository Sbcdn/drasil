/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use chrono::{DateTime, Utc};

use bigdecimal::BigDecimal;
use sleipnir::rewards::models::WhitelistLink;
use std::fmt;

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub(crate) enum AddrSrc {
    GPools(gungnir::GPools),
    Whitelist(WhitelistLink),
}
impl fmt::Display for AddrSrc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::GPools(p) => {
                write!(f, "{},", p)
            }
            Self::Whitelist(w) => {
                write!(f, "{},", w)
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct TwlData {
    pub fingerprint: String,
    pub policy_id: String,
    pub tokenname: String,
    pub contract_id: i64,
    pub user_id: i64,
    pub vesting_period: DateTime<Utc>,
    pub addr_src: AddrSrc,
    pub mode: gungnir::Calculationmode,
    pub equation: String,
    pub start_epoch: i64,
    pub end_epoch: Option<i64>,
    pub modificator_equ: Option<String>,
    pub calc_epoch: i64,
}

impl TwlData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        fingerprint: String,
        policy_id: String,
        tokenname: String,
        contract_id: i64,
        user_id: i64,
        vesting_period: DateTime<Utc>,
        addr_src: AddrSrc,
        mode: gungnir::Calculationmode,
        equation: String,
        start_epoch: i64,
        end_epoch: Option<i64>,
        modificator_equ: Option<String>,
        calc_epoch: i64,
    ) -> TwlData {
        TwlData {
            fingerprint,
            policy_id,
            tokenname,
            contract_id,
            user_id,
            vesting_period,
            addr_src,
            mode,
            equation,
            start_epoch,
            end_epoch,
            modificator_equ,
            calc_epoch,
        }
    }

    pub fn to_str_vec(&self) -> Vec<String> {
        vec![
            self.fingerprint.clone(),
            self.policy_id.clone(),
            self.tokenname.clone(),
            self.contract_id.to_string(),
            self.user_id.to_string(),
            self.vesting_period.to_string(),
            self.addr_src.to_string(),
            self.mode.to_string(),
            self.equation.clone(),
            self.start_epoch.to_string(),
            self.end_epoch.unwrap_or(0).to_string(),
            self.modificator_equ
                .clone()
                .unwrap_or_else(|| (&"None").to_string()),
            self.calc_epoch.to_string(),
        ]
    }
}
impl fmt::Display for TwlData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},", self.fingerprint)?;
        write!(f, "{},", self.policy_id)?;
        write!(f, "{},", self.tokenname)?;
        write!(f, "{},", self.contract_id)?;
        write!(f, "{},", self.user_id)?;
        write!(f, "{},", self.vesting_period)?;
        write!(f, "{},", self.addr_src)?;
        write!(f, "{},", self.mode.to_string())?;
        write!(f, "{},", self.equation)?;
        write!(f, "{},", self.start_epoch)?;
        write!(f, "{},", self.end_epoch.unwrap_or(0))?;
        write!(
            f,
            "{},",
            self.modificator_equ.as_ref().unwrap_or(&"None".to_string())
        )?;
        write!(f, "{}", self.calc_epoch)
    }
}
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct RewardTable {
    pub twldata: TwlData,
    pub calc_date: DateTime<Utc>,
    pub calc_epoch: i64,
    pub current_epoch: i64,
    pub earned_epoch: BigDecimal,
    pub total_earned_epoch: BigDecimal,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;
