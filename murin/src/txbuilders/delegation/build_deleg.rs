/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr , utils as cutils};
use crate::hfn::{balance_tx,get_ttl_tx,sum_output_values,get_vkey_count,calc_txfee,make_dummy_vkeywitnesses,tx_output_data};
use crate::htypes::*;
use crate::error::MurinError;
use crate::{TxData};
use crate::txbuilders::delegation::DelegTxData; 
use crate::txbuilders::{input_selection};



fn perform_delegation (   
    fee                     : &cutils::BigNum, 
    gtxd                    : &TxData,
    pooltxd                 : &DelegTxData,
    registered              : &bool,
    dummy                   : bool
    ) -> std::result::Result<(clib::TransactionBody, clib::TransactionWitnessSet, clib::metadata::AuxiliaryData, TransactionUnspentOutputs, usize),MurinError> {
        
        if dummy == true {
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Fee calcualtion------------------------------------------------");
            info!("---------------------------------------------------------------------------------------------------------\n"); 
        } else {
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Build Transaction----------------------------------------------");
            info!("--------------------------------------------------------------------------------------------------------\n");
        }

        dotenv::dotenv().ok();

       
        
        let owner_address = match gtxd.get_senders_address(None) {
            Some(a) => a,
            None => return Err(MurinError::new("Address of Wallet owner could not be found"))
        };
        let delegators_address     : caddr::Address = gtxd.get_stake_address();

        let delegators_address_bech32 = delegators_address.to_bech32(None)?;
        info!("Delegator Stake Address: {:?}",delegators_address_bech32);

        let owner_base_addr = caddr::BaseAddress::from_address(&owner_address).unwrap();
        let owner_stakecred = owner_base_addr.stake_cred();
        let deleg_rwd_addr =  caddr::RewardAddress::from_address(&delegators_address).unwrap();
        let deleg_stake_creds = deleg_rwd_addr.payment_cred();
        if owner_stakecred.to_bytes() != deleg_stake_creds.to_bytes() {
            return Err(MurinError::new("Inconsitent Stake Key Data, forbidden!"))
        }

        let mut certs = clib::Certificates::new();
        if !*registered {
            let stake_reg = clib::StakeRegistration::new(&deleg_stake_creds);
            let reg_cert = clib::Certificate::new_stake_registration(&stake_reg);
            certs.add(&reg_cert);
        }   

        let stake_delegation = clib::StakeDelegation::new(&deleg_stake_creds, &pooltxd.get_poolkeyhash());
        let deleg_cert = clib::Certificate::new_stake_delegation(&stake_delegation);  
        certs.add(&deleg_cert);


        /////////////////////////////////////////////////////////////////////////////////////////////////////
        //
        //Auxiliary Data
        //  Plutus Script and Metadata
        /////////////////////////////////////////////////////////////////////////////////////////////////////
        let aux_data = clib::metadata::AuxiliaryData::new();   
        //let aux_data_hash = cutils::hash_auxiliary_data(&aux_data);
        

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////
        
        let mut txouts = clib::TransactionOutputs::new(); 
        let deposit_val = cutils::Value::new(&cutils::to_bignum(2000000));
            
        // Inputs
        let mut input_txuos = gtxd.clone().get_inputs();

        info!("\n Before USED UTXOS");
        // Check if some utxos in inputs are in use and remove them
        if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
            info!("\n\n");
            info!("USED UTXOS: {:?}",used_utxos);
            info!("\n\n");
            input_txuos.remove_used_utxos(used_utxos);
        } 


        let k = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)?;
        info!("K: {:?}",k);

        let collateral_input_txuo  = gtxd.clone().get_collateral();        
        debug!("\nCollateral Input: {:?}",collateral_input_txuo);

        // Balance TX
        debug!("Before Balance: Transaction Inputs: {:?}",input_txuos);
        debug!("Before Balance: Transaction Outputs: {:?}",txouts);

        let mut fee_paied = false;
        let mut first_run = true;
        let mut txos_paied = false;
        let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64));    
        if !*registered {
            tbb_values = deposit_val.clone();          
        }
        let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));                         
        let change_address = owner_address.clone();    
        
        let mut needed_value = sum_output_values(&txouts);
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
        
        if !*registered {
            needed_value = needed_value.checked_add(&deposit_val)?;
        }
        
        let security = cutils::to_bignum(cutils::from_bignum(&needed_value.coin()) / 100 * 10 + (1*MIN_ADA)); // 10% Security for min utxo etc. 
        needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());
        let mut needed_value = cutils::Value::new(&needed_value.coin());

        debug! ("Needed Value: {:?}",needed_value);
        debug!("\n\n\n\n\nTxIns Before selection:\n {:?}\n\n\n\n\n",input_txuos);
    
        // !!!! CHeck if input selection just tries to find ADA!!!!
        let (txins, mut input_txuos) = 
            input_selection(
                None, 
                &mut needed_value, 
                &input_txuos, 
                gtxd.clone().get_collateral(),
                None,//Some(native_script_address).as_ref(),
        )?;

        let saved_input_txuos = input_txuos.clone();
        info!("Saved Inputs: {:?}",saved_input_txuos);

        let vkey_counter = get_vkey_count(&input_txuos, collateral_input_txuo.as_ref()) + 1; // +1 dues to signature in finalize
        debug!("\n\n\n\n\nTxIns Before Balance:\n {:?}\n\n\n\n\n",input_txuos);

        let txouts_fin = balance_tx(
            &mut input_txuos,             
            &Tokens::new(),         
            &mut txouts,
            None, // but not the ADA!!!!                   
            fee,                           
            &mut fee_paied,                
            &mut first_run,                
            &mut txos_paied,
            &mut tbb_values,
            &owner_address,
            &change_address,         
            &mut acc,
            None,
            &dummy
            )?;

            
        let slot = gtxd.clone().get_current_slot() + get_ttl_tx(&gtxd.clone().get_network());
        let mut txbody = clib::TransactionBody::new(&txins,&txouts_fin,&fee,Some(slot as u32));
        info!("\nTxOutputs: {:?}\n",txbody.outputs());
        debug!("\nTxInputs: {:?}\n",txbody.inputs());

        //txbody.set_auxiliary_data_hash(&aux_data_hash);
        txbody.set_certs(&certs);
        
         // Set network Id
        if gtxd.get_network() == clib::NetworkIdKind::Testnet {
            txbody.set_network_id(&clib::NetworkId::testnet());
        }else{
            txbody.set_network_id(&clib::NetworkId::mainnet());
        }  


        let txwitness = clib::TransactionWitnessSet::new();  

        debug!("TxWitness: {:?}", hex::encode(txwitness.to_bytes()));
        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        debug!("Vkey Counter at End: {:?}",vkey_counter);
        Ok((txbody,txwitness,aux_data,saved_input_txuos,vkey_counter))
}

//ToDO:
// - AutoMint: The Minting scripts signs itself and submits transaction directly, there need to bw enough funds on the script address
// - Manage on Script: 
//      - Send Minted Tokens to this scriptaddress
//      - Allows burning of tokens from this script address
//      - Allows checked mint against existing tokennames
//      - Needs extended admin functions as sending all tokens to a receiving address -> Can be used for payment of the service
//      - In Admin panel you need to see all prepared mints and oyu mint 20 per transactions. -> Automate process in Admin panel


pub async fn build_delegation_tx (
            gtxd : &TxData, 
            delegtxd : &DelegTxData, 
            registered : &bool,
        ) -> std::result::Result<crate::htypes::BuildOutput, MurinError> {
              // Temp until Protocol Parameters fixed
            let mem = cutils::to_bignum(7000000u64);        //cutils::to_bignum(7000000u64);    
            let steps = cutils::to_bignum(2500000000u64);   //cutils::to_bignum(3000000000u64);
            let ex_unit_price : crate::htypes::ExUnitPrice = crate::ExUnitPrice { priceSteps: 7.21e-5, priceMemory:  5.77e-2 };
            //serde_json::from_str("{'executionUnitPrices': {'priceSteps': 7.21e-5,'priceMemory': 5.77e-2}").unwrap();
            let a = cutils::to_bignum(44u64);
            let b = cutils::to_bignum(155381u64);

            //

        //Create Tx
            let (txbody_,mut txwitness_, aux_data_, _,vkey_counter) = 
            perform_delegation(&cutils::to_bignum(2000000), gtxd, delegtxd, registered, true)?;

            let dummy_vkeywitnesses = make_dummy_vkeywitnesses(vkey_counter); 
            txwitness_.set_vkeys(&dummy_vkeywitnesses);

            // Build and encode dummy transaction
            let transaction_ = clib::Transaction::new(&txbody_,&txwitness_,Some(aux_data_.clone()));

            let calculated_fee = calc_txfee(&transaction_,&a,&b,ex_unit_price.clone(),&steps,&mem,true);   
            let (txbody, txwitness, aux_data,used_utxos,vkey_counter_2) = 
            perform_delegation(&calculated_fee,  gtxd, delegtxd, registered, true)?;

            let transaction2 = clib::Transaction::new(&txbody,&txwitness_,Some(aux_data.clone()));


            if vkey_counter_2 != vkey_counter || transaction2.to_bytes().len() != transaction_.to_bytes().len() {
                let dummy_vkeywitnesses = make_dummy_vkeywitnesses(vkey_counter_2); 
                txwitness_.set_vkeys(&dummy_vkeywitnesses);

                let calculated_fee = calc_txfee(&transaction2,&a,&b,ex_unit_price,&steps,&mem,true);   
                let (txbody, txwitness, aux_data,used_utxos,_) = 
                perform_delegation(&calculated_fee,  gtxd, delegtxd, registered, true)?;
                info!("Fee: {:?}",calculated_fee);
                debug!("\n\n\nDummy vkey_counter: {:?} \n\n",vkey_counter);
                debug!("\n\n\nDummy vkey_counter_2: {:?} \n\n",vkey_counter_2);
                Ok(tx_output_data (txbody,txwitness,aux_data,used_utxos.to_hex()?,0u64,false)?)
            } else {
            info!("Fee: {:?}",calculated_fee);
                Ok(tx_output_data(txbody,txwitness,aux_data,used_utxos.to_hex()?,0u64,false)?)
            }            

        }