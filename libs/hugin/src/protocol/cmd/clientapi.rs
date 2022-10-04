/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::{Parse, CmdError};
use crate::{Connection,Frame,IntoFrame};
use crate::datamodel::{MultiSigType,TransactionPattern,ScriptSpecParams};

use bytes::Bytes;
use mimir::MurinError;
use serde_json::json;
use tracing::{debug, instrument};
use bincode as bc;
use bc::Options;

use rand::Rng;

#[derive(Debug,Clone)]
pub struct ClientApi {
    customer_id : u64, 
    request_type : APIRequestType,
    request_payload : ApiRequestPayloadType, 
}