/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
#![warn(unused_assignments)]
use hugin::database::drasildb::*;

extern crate  diesel;
use std::io::{stdin};

fn main() -> Result<(),murin::MurinError> {
    let connection = establish_connection()?;

    

    println!("Please provide id (i64):");
    let mut db_id = String::new();
    stdin().read_line(&mut db_id).unwrap();
    let db_id = &db_id[..(db_id.len() - 1)];
    let db_id = db_id.parse::<i64>()?;

    let contract_org = TBContracts::get_contract_by_id(&connection, db_id)?;

    println!("Please provide contract-id (i64):");
    println!("Leave empty to keep current value.");
    let mut contract_id = String::new();
    stdin().read_line(&mut contract_id).unwrap();
    let contract_id_ = &contract_id[..(contract_id.len() - 1)];
    let mut contract_id = contract_org.contract_id;
    if contract_id_ != "" {
        contract_id = contract_id_.parse::<i64>()?;
    } 

    println!("Please provide a description (optional):");
    println!("Leave empty to keep current value.");
    let mut description_ = String::new();
    stdin().read_line(&mut description_).unwrap();
    let description_ = &description_[..(description_.len() - 1)];
    
    let mut description  : Option<&str> = None;
    let mut k = String::new();
    if let Some(org_description) = contract_org.description {
        k = org_description;
        description = Some(&k);
    };
    
    if description_ != "" {
        description = Some(&description_)
    }

    println!("You want to depricate the contract? (true / false):");
    println!("Leave empty to keep current value.");
    let mut depri_ = String::new();
    stdin().read_line(&mut depri_).unwrap();
    let depri_ = &depri_[..(depri_.len() - 1)];
    let mut depri = contract_org.depricated;
    if depri_ != "" {
        depri = depri_.parse::<bool>()?;
    }
    let contract = TBContracts::update_contract(
        &connection, &db_id, &contract_id, description, &depri)?;


    println!("\n\n Success! updated: \n {:?}",contract.id);
    Ok(())
}
