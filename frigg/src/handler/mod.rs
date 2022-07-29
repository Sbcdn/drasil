/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

pub mod rwd;
pub mod dapi;

use crate::Result;
use crate::error::Error;

pub async fn get_user_from_string(us : &String) -> Result<i64> {
    let user = match us.parse::<i64>() {
        Ok(u) => u,
        Err(_) => {
            return Err(Error::Custom("invalid user".to_string()))
        }
    };

    Ok(user)
}