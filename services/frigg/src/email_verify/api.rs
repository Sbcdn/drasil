/*
#################################################################################
# Business Source License           See LICENSE.md for full license information.#
# Licensor:             Drasil Blockchain Association                           #
# Licensed Work:        Drasil Application Framework v.0.2. The Licensed Work   #
#                       is © 2022 Drasil Blockchain Association                 #
# Additional Use Grant: You may use the Licensed Work when your application     #
#                       using the Licensed Work is generating less than         #
#                       $150,000 and the entity operating the application       #
#                       engaged equal or less than 10 people.                   #
# Change Date:          Drasil Application Framework v.0.2, change date is two  #
#                       and a half years from release date.                     #
# Change License:       Version 2 or later of the GNU General Public License as #
#                       published by the Free Software Foundation.              #
#################################################################################
*/

pub use crate::email_verify::*;
pub use crate::error::Error;
pub use hugin::database::{TBEmailVerificationToken, TBEmailVerificationTokenMessage};

#[derive(Debug, serde::Serialize)]
pub struct Msg {
    message: String,
}
impl Msg {
    pub fn new(msg: String) -> Self {
        Msg { message: msg }
    }
}

#[derive(serde::Deserialize)]
pub struct RegistrationMessage {
    token: String,
    email: String,
}

pub async fn invite(body: TBEmailVerificationTokenMessage) -> crate::WebResult<impl warp::Reply> {
    log::debug!("invite");
    let token = match TBEmailVerificationToken::create(body.clone()) {
        Ok(t) => t,
        Err(_) => {
            return Err(warp::reject::custom(Error::Custom(
                "Could not create verification token".to_string(),
            )))
        }
    };
    let token_string = hex::encode(token.id);
    let uname = match body.id {
        Some(n) => n,
        None => "Drasil User".to_string(),
    };

    let link = std::env::var("VERIFICATION_LINK").unwrap();

    Email::new(Contact::new("verify@drasil.io", "Drasil E-Mail Verification"), Contact::new(body.email.clone(), uname.clone()))
        .set_subject("Confirm Your Email")
        .set_html(format!("Dear {},\n\nYou get this email because you registered on Drasil.io, if you did not please contact us.\n\nYour confirmation code is: {} \n\nPlease go to {} and enter your email address and confirmation code.\n\nThank You\nThe Drasil Team", uname,&token_string,link))
        .send().await?;

    Ok(warp::reply::with_status(
        warp::reply::json(&Msg::new(format!(
            "User successfully registered, please verify e-mail address to proceed. E-Mail: {}",
            body.email
        ))),
        warp::http::StatusCode::OK,
    ))
}

pub async fn verify(body: RegistrationMessage) -> crate::WebResult<impl warp::Reply> {
    let token_id =
        hex::decode(body.token).map_err(|_| Error::Custom("Invalid token".to_string()))?;

    let token = TBEmailVerificationToken::find(&token_id)
        .map_err(|_| Error::Custom("Invalid token".to_string()))?;

    if token.email != body.email {
        return Err(warp::reject::custom(Error::Custom(
            "Invalid token".to_string(),
        )));
    }

    if token.expires_at < chrono::Utc::now() {
        return Err(warp::reject::custom(Error::Custom(
            "Invalid token".to_string(),
        )));
    }

    let _ = match hugin::drasildb::TBDrasilUser::verify_email(&token.email) {
        Ok(u) => u,
        Err(_) => {
            return Err(warp::reject::custom(Error::Custom(
                "Could not update user please retry the verification process".to_string(),
            )))
        }
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&Msg::new("Successfully verified".to_string())),
        warp::http::StatusCode::OK,
    ))
}
