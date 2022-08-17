/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use serde::{Deserialize, Serialize};

use strum_macros::EnumString;

#[derive(EnumString, Serialize, Deserialize, Debug, Clone)]
pub(crate) enum CustomCalculationTypes {
    Freeloaderz,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct FreeloaderzType {
    pub min_stake: i32,
    pub min_earned: f64,
    pub flatten: f64,
}
