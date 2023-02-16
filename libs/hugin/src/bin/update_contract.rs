/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
#![warn(unused_assignments)]
use hugin::{database::drasildb::*, error::SystemDBError};

extern crate diesel;
use std::io::stdin;

fn main() -> Result<(), SystemDBError> {
    println!("Please provide id (i64):");
    let mut db_id = String::new();
    stdin().read_line(&mut db_id).unwrap();
    let db_id = &db_id[..(db_id.len() - 1)];
    let db_id = db_id.parse::<i64>()?;

    let contract_org = TBContracts::get_contract_by_id(db_id)?;

    println!("Please provide contract-id (i64):");
    println!("Leave empty to keep current value.");
    let mut contract_id = String::new();
    stdin().read_line(&mut contract_id).unwrap();
    let contract_id_ = &contract_id[..(contract_id.len() - 1)];
    let mut contract_id = contract_org.contract_id;
    if !contract_id_.is_empty() {
        contract_id = contract_id_.parse::<i64>()?;
    }

    println!("Please provide a description (optional):");
    println!("Leave empty to keep current value.");
    let mut description = String::new();
    stdin().read_line(&mut description).unwrap();
    let description = &description[..(description.len() - 1)];

    println!("You want to depricate the contract? (true / false):");
    println!("Leave empty to keep current value.");
    let mut depri_ = String::new();
    stdin().read_line(&mut depri_).unwrap();
    let depri_ = &depri_[..(depri_.len() - 1)];
    let mut depri = contract_org.depricated;
    if !depri_.is_empty() {
        depri = depri_.parse::<bool>()?;
    }
    let contract = TBContracts::update_contract(
        &db_id,
        &contract_id,
        contract_org
            .description
            .as_ref()
            .map(|x| Some(&**x))
            .unwrap_or(Some(description)),
        &depri,
    )?;

    println!("\n\n Success! updated: \n {:?}", contract.id);
    Ok(())
}
