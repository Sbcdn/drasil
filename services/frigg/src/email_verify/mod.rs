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

mod api;
use crate::error::Error;
pub use api::*;
use serde::Serialize;

use lettre::Message;
use rusoto_ses::{RawMessage, SendRawEmailRequest, Ses, SesClient};

lazy_static::lazy_static! {
    static ref SMTP_USER: String = std::env::var("SMTP_USER").unwrap_or_else(|_| "".to_string());
    static ref SMTP_PW: String = std::env::var("SMTP_PW").unwrap_or_else(|_| "".to_string());
}

#[derive(Debug, Serialize)]
pub struct Contact {
    email: String,
    name: Option<String>,
}

impl Contact {
    pub fn new<T: Into<String>>(email: T, name: T) -> Self {
        Contact {
            email: email.into(),
            name: Some(name.into()),
        }
    }
}

impl<T: Into<String>> From<T> for Contact {
    fn from(email: T) -> Self {
        Contact {
            email: email.into(),
            name: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Email {
    sender: Contact,
    recipient: Contact,
    subject: String,
    html: String,
}

impl Email {
    pub fn new(sender: Contact, recipient: Contact) -> Self {
        Email {
            sender,
            recipient,
            subject: "".to_string(),
            html: "".to_string(),
        }
    }

    pub fn _add_recipient<T: Into<Contact>>(mut self, recipient: T) -> Self {
        self.recipient = recipient.into();
        self
    }

    pub fn set_subject<T: Into<String>>(mut self, subject: T) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn set_html<T: Into<String>>(mut self, html: T) -> Self {
        self.html = html.into();
        self
    }
    pub async fn send(self) -> Result<String, Error> {
        let ses_client = SesClient::new(rusoto_core::Region::UsEast2);
        let mut rname = String::new();
        if let Some(name) = self.recipient.name {
            rname = name;
        }
        let email = Message::builder()
            .to((rname + "<" + &self.recipient.email + ">").parse().unwrap())
            .from(std::env::var("FROM_EMAIL").unwrap().parse().unwrap())
            .subject(self.subject)
            .body(self.html)
            .unwrap();

        let raw_email = email.formatted();
        let ses_request = SendRawEmailRequest {
            raw_message: RawMessage {
                data: base64::encode(raw_email).into(),
            },
            source_arn: Some(std::env::var("EMAIL_API_KEY").unwrap()),
            ..Default::default()
        };

        let mailer = ses_client.send_raw_email(ses_request).await;

        match mailer {
            Ok(_) => Ok("Email sent successfully!".to_string()),
            Err(e) => Err(Error::Custom(format!("Could not send email: {:?}", e))),
        }
    }
}
