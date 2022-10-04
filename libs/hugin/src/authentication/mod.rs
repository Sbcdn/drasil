/*
#################&################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
/*
use google_authenticator::*;
use murin::MurinError;

use crate::encryption;

pub fn test() -> std::result::Result<(), MurinError> {
    let secret = create_secret!();
    let code = get_code!(&secret).unwrap();
    let verify = verify_code!(&secret, &code);

    Ok(())
}

pub fn create_secret(user_id: &i64, pw: &String) -> std::result::Result<(), MurinError> {
    let user = crate::TBDrasilUser::verify_pw_userid(user_id, pw)?;
    let ident = hex::encode(
        &murin::crypto::PublicKey::from_bech32(&user.drslpubkey)?
            .hash()
            .to_bytes(),
    );
    encryption::vault_get(&ident);

    Ok(())
}
*/
