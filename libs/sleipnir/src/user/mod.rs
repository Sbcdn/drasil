/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
pub use crate::error::SleipnirError;
use chrono::{DateTime, Utc};
use hugin::TBCaPayment;
use murin::utils::from_bignum;

pub async fn create_rev_payout(
    user_id: i64,
    contract_id: i64,
    pw: String,
) -> Result<hugin::TBCaPayment, SleipnirError> {
    let contract = hugin::TBContracts::get_contract_uid_cid(user_id, contract_id)?;

    let mut mconn = mimir::establish_connection()?;
    let address_utxos = mimir::get_address_utxos(&mut mconn, &contract.address)?;

    let total_value = address_utxos.calc_total_value()?;
    let contract_lqdty = contract.get_contract_liquidity();
    let payout_value = hugin::CaValue::new(
        murin::utils::from_bignum(&total_value.coin().checked_sub(&contract_lqdty)?),
        vec![],
    );
    Ok(hugin::TBCaPayment::create(&user_id, &contract_id, &payout_value, &pw).await?)
}

pub async fn create_custom_payout(
    user_id: i64,
    contract_id: i64,
    ada_value: i64,
    tokens: Vec<hugin::Token>,
    pw: String,
) -> Result<hugin::TBCaPayment, SleipnirError> {
    log::debug!("Try to get contract...");
    let contract = hugin::TBContracts::get_contract_uid_cid(user_id, contract_id)?;

    log::debug!("Try to connect to dbsync...");
    let mut mconn = mimir::establish_connection()?;
    log::debug!("Try to find contract utxos...");
    let address_utxos = mimir::get_address_utxos(&mut mconn, &contract.address)?;
    log::debug!("Try to calculate total value...");
    let total_value = address_utxos.calc_total_value()?;
    log::debug!("Try to get contract liquidity...");
    let contract_lqdty = contract.get_contract_liquidity();
    log::debug!("Try to calculate difference...");
    let diff = from_bignum(&total_value.coin().checked_sub(&contract_lqdty).unwrap());
    log::debug!("Check payout and liquiditity value...");
    if diff < ada_value as u64 || ada_value <= 0 {
        return Err(SleipnirError::new(
            "Requested Ada is not available on this contract",
        ));
    }
    log::debug!("Create payout value...");

    let payout_value = hugin::CaValue::new(ada_value as u64, tokens);
    log::debug!("All check passed, create on database...");
    Ok(hugin::TBCaPayment::create(&user_id, &contract_id, &payout_value, &pw).await?)
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct DispCaPayment {
    pub id: i64,
    pub contract_id: i64,
    pub value: String,
    pub tx_hash: Option<String>,
    pub user_appr: bool,
    pub drasil_appr: bool,
    pub stauts_bl: Option<String>,
    pub stauts_pa: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DispCaPayment {
    pub fn from_cap(caps: Vec<hugin::TBCaPayment>) -> Vec<Self> {
        let mut out = Vec::<DispCaPayment>::new();
        caps.iter().for_each(|n| {
            out.push(DispCaPayment {
                id: n.id,
                contract_id: n.contract_id,
                value: n.value.to_owned(),
                tx_hash: n.tx_hash.to_owned(),
                user_appr: n.user_appr.is_some(),
                drasil_appr: n.drasil_appr.is_some(),
                stauts_bl: n.stauts_bl.to_owned(),
                stauts_pa: n.stauts_pa.to_owned(),
                created_at: n.created_at,
                updated_at: n.updated_at,
            })
        });
        out
    }
}

pub async fn show_payouts(user_id: i64) -> Result<Vec<DispCaPayment>, SleipnirError> {
    let caps =
        hugin::TBCaPayment::find_all(&user_id).map_err(|e| SleipnirError::new(&e.to_string()))?;
    Ok(DispCaPayment::from_cap(caps))
}

pub async fn find_payout(user_id: i64, po_id: i64) -> Result<TBCaPayment, SleipnirError> {
    let caps = hugin::TBCaPayment::find(&po_id).map_err(|e| SleipnirError::new(&e.to_string()))?;
    if caps.user_id != user_id {
        return Err(SleipnirError::new("wrong user"));
    }
    Ok(caps)
}

// ToDO: TWO FACTOR AUTHENTICATION
pub async fn approve_payout(
    user_id: &i64,
    payout_id: &i64,
    pw: &String,
    _mfa: &str,
) -> Result<(), SleipnirError> {
    let user = hugin::TBDrasilUser::get_user_by_user_id(user_id)?;

    let msg = hugin::TBCaPaymentHash::find_by_payid(payout_id)?[0]
        .payment_hash
        .clone();
    let payment = hugin::TBCaPayment::find(payout_id)?;
    if payment.user_id != *user_id || payment.hash().await? != msg {
        return Err(SleipnirError::new("Error: POT1201"));
    }
    let signature = user.approve(pw, &msg).await?;

    payment.approve_user(&signature).await?;

    Ok(())
}
