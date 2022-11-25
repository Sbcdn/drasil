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

pub mod adm;
pub mod dapi;
pub mod discounts;
pub mod mint;
pub mod rwd;

use crate::error::Error;
use crate::Result;

pub async fn get_user_from_string(us: &str) -> Result<i64> {
    let user = match us.parse::<i64>() {
        Ok(u) => u,
        Err(_) => return Err(Error::Custom("invalid user".to_string())),
    };

    Ok(user)
}
