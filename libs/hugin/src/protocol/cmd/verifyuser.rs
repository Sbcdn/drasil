use crate::{CmdError, Parse};
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct VerifyUser {
    user_id: u64,
    bearer_token: String,
}

impl VerifyUser {
    pub fn new(cid: u64, btoken: String) -> VerifyUser {
        VerifyUser {
            user_id: cid,
            bearer_token: btoken,
        }
    }

    pub fn customer_id(&self) -> u64 {
        self.user_id
    }

    pub fn tx_type(&self) -> String {
        self.bearer_token.clone()
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<VerifyUser> {
        let customer_id = parse.next_int()?;
        let btoken = parse.next_bytes()?;
        let btoken: String = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&btoken)?;
        Ok(VerifyUser {
            user_id: customer_id,
            bearer_token: btoken,
        })
    }

    /// Apply verify user
    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let user = crate::database::TBDrasilUser::get_user_by_user_id(&(self.user_id as i64))?;

        if let Some(token) = user.api_pubkey {
            if token == self.bearer_token {
                let response = Frame::Bulk(Bytes::from(
                    bc::DefaultOptions::new()
                        .with_varint_encoding()
                        .serialize(&"true".to_string())?,
                ));
                dst.write_frame(&response).await?;
            } else {
                return Err(CmdError::Custom {
                    str: "ERROR not authenticated".to_string(),
                }
                .into());
            }
        }
        Ok(())
    }
}

impl IntoFrame for VerifyUser {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("vus".as_bytes()));

        frame.push_int(self.user_id);

        let mtype_b = bc::DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&self.bearer_token)
            .unwrap();
        frame.push_bulk(Bytes::from(mtype_b));

        frame
    }
}
