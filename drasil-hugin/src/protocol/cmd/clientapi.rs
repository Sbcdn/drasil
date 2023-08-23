use crate::datamodel::{MultiSigType, ScriptSpecParams, TransactionPattern};
use crate::{CmdError, Parse};
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;
use drasil_mimir::MurinError;
use serde_json::json;
use tracing::{debug, instrument};

use rand::Rng;

#[derive(Debug, Clone)]
pub struct ClientApi {
    customer_id: u64,
    request_type: APIRequestType,
    request_payload: ApiRequestPayloadType,
}
