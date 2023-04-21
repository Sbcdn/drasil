/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use crate::drasildb::error::SystemDBError;
use dvltath::vault::kv::vault_get;

// ToDO: TWO FACTOR AUTHENTICATION
pub async fn approve_payout_drsl(
    payout_id: &i64,
    pw: &String,
    _mfa: &str,
) -> Result<(), SystemDBError> {
    let user_id = vault_get(&std::env::var("ADM_USER").expect("Error: A1201"))
        .await
        .get("user")
        .expect("Error: A1202")
        .parse::<i64>()?;
    let user = crate::TBDrasilUser::get_user_by_user_id(&user_id)?;

    let msg = crate::TBCaPaymentHash::find_by_payid(payout_id)?[0]
        .payment_hash
        .clone();
    let payment = crate::TBCaPayment::find(payout_id)?;
    if payment.hash().await? != msg {
        return Err(SystemDBError::Custom("Error: A1203".to_string()));
    }
    let signature = user.approve(pw, &msg).await?;

    payment.approve_drasil(&signature).await?;

    Ok(())
}

pub async fn verify_approval_drsl(msg: &str, sign: &str) -> Result<bool, SystemDBError> {
    let user_id = vault_get(&std::env::var("ADM_USER").expect("Error: A1201"))
        .await
        .get("user")
        .expect("Error: A1202")
        .parse::<i64>()?;
    let user = crate::TBDrasilUser::get_user_by_user_id(&user_id)?;

    let pk = murin::crypto::PublicKey::from_bech32(&user.drslpubkey)?;
    let sign = murin::crypto::Ed25519Signature::from_hex(sign)?;
    if !pk.verify(msg.as_bytes(), &sign) {
        return Err(SystemDBError::Custom("Error: A0010".to_string()));
    }
    Ok(true)
}

// ToDO: TWO FACTOR AUTHENTICATION
pub async fn get_vaddr(_user: &i64) -> Result<String, SystemDBError> {
    // ToDo:
    // Get useres idvidually verified wallet for this user

    //let user = TBDrasilUser::get_user_by_user_id(&mut crate::establish_connection()?, user)?;

    /*
    let vaddr = vault_get(&user.drslpubkey)
        .await
        .get("vaddr")
        .expect("Error: A1205: Could not retrieve verified address")
        .clone();
    */

    //Get general admin wallet for payouts
    let vaddr = std::env::var("POW").unwrap();

    Ok(vaddr)
}
