/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use murin::MurinError;

// ToDO: TWO FACTOR AUTHENTICATION
pub async fn approve_payout_drsl(
    payout_id: &i64,
    pw: &String,
    mfa: &String,
) -> Result<(), MurinError> {
    let dconn = crate::establish_connection()?;
    let user_id = crate::encryption::vault_get(&std::env::var("ADM_USER").expect("Error: A1201"))
        .await
        .get("user")
        .expect("Error: A1202")
        .parse::<i64>()?;
    let user = crate::TBDrasilUser::get_user_by_user_id(&dconn, &user_id)?;

    let msg = crate::TBCaPaymentHash::find_by_payid(payout_id)?[0]
        .payment_hash
        .clone();
    let payment = crate::TBCaPayment::find(payout_id)?;
    if payment.hash()? != msg {
        return Err(MurinError::new("Error: A1203"));
    }
    let signature = user.approve(pw, &msg).await?;

    payment.approve_drasil(&signature)?;

    Ok(())
}

pub async fn verify_approval_drsl(msg: &str, sign: &str) -> Result<bool, MurinError> {
    let dconn = crate::establish_connection()?;
    let user_id = crate::encryption::vault_get(&std::env::var("ADM_USER").expect("Error: A1201"))
        .await
        .get("user")
        .expect("Error: A1202")
        .parse::<i64>()?;
    let user = crate::TBDrasilUser::get_user_by_user_id(&dconn, &user_id)?;

    let pk = murin::crypto::PublicKey::from_bech32(&user.drslpubkey)?;
    let sign = murin::crypto::Ed25519Signature::from_hex(sign)?;
    if !pk.verify(msg.as_bytes(), &sign) {
        return Err(MurinError::new("Error: A0010"));
    }
    Ok(true)
}
