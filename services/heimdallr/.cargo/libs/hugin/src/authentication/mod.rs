use std::fmt;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Role {
    StandardUser,
    EnterpriseUser,
    Retailer,
    DrasilAdmin,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::DrasilAdmin => write!(f, "0"),
            Role::Retailer => write!(f, "1"),
            Role::EnterpriseUser => write!(f, "2"),
            Role::StandardUser => write!(f, "3"),
        }
    }
}

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
