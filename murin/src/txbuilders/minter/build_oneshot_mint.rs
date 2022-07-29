/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto, utils as cutils};
use clib::address::Address;
use crate::hfn::{balance_tx,get_ttl_tx,sum_output_values,get_vkey_count,calc_txfee,make_dummy_vkeywitnesses,tx_output_data};
use crate::htypes::*;
use crate::error::MurinError;
use crate::{TxData};
use crate::txbuilders::minter::MinterTxData; 
use crate::txbuilders::{calc_min_ada_for_utxo,harden,input_selection};
use crate::minter::*;

pub fn perform_mint ( 
    fee                     : &cutils::BigNum, 
    native_script           : &clib::NativeScript,
    gtxd                    : &TxData,
    minttxd                 : &MinterTxData,
    pvks                    : &Vec::<String>,
    liquidity_addr          : &Address,
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

        let receiving_address     : caddr::Address = minttxd.get_payment_addr();

        
        let receiving_address_bech32 = receiving_address.to_bech32(None)?;
        debug!("Recipent Address: {:?}",receiving_address_bech32);

        let mintpolicy = native_script.hash(); 
        let minttokens = mintasset_into_tokenasset(minttxd.get_mint_tokens(),mintpolicy.clone());


        /////////////////////////////////////////////////////////////////////////////////////////////////////
        //
        //Auxiliary Data
        //  Plutus Script and Metadata
        /////////////////////////////////////////////////////////////////////////////////////////////////////
        let mut aux_data = clib::metadata::AuxiliaryData::new();   
        let metadata = make_mint_metadata_from_json(&minttxd.get_metadata(),minttokens.clone(), mintpolicy.clone())?;
        aux_data.set_metadata(&metadata);
        let aux_data_hash = cutils::hash_auxiliary_data(&aux_data);


        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////
        
        let mut txouts = clib::TransactionOutputs::new(); 
        let _zero_val = cutils::Value::new(&cutils::to_bignum(0u64));
      
        let mut mint_val = tokens_to_value(&minttokens.clone());
        let min_utxo_val = calc_min_ada_for_utxo(&mint_val,None);
        mint_val.set_coin(&min_utxo_val);


        txouts.add(&clib::TransactionOutput::new(&receiving_address, &mint_val));
               
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
        let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));                         
        let change_address = liquidity_addr.clone();    
        
        let mut needed_value = sum_output_values(&txouts);
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
        let security = cutils::to_bignum(cutils::from_bignum(&needed_value.coin()) / 100 * 10 + (2*MIN_ADA)); // 10% Security for min utxo etc. 
        needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());
        let mut needed_value = cutils::Value::new(&needed_value.coin());

        debug! ("Needed Value: {:?}",needed_value);
        debug!("\n\n\n\n\nTxIns Before selection:\n {:?}\n\n\n\n\n",input_txuos);
      
        //let mut signers_address_utxos = (TransactionUnspentOutputs::new(),TransactionUnspentOutputs::new());
        //if let Some(signer) = minttxd.get_signer() {
        //    signers_address_utxos = find_utxos_by_address(signer.clone(), &input_txuos);
        //}

        //println!("Signer Address UTXOS: {:?}",signers_address_utxos.0);
        let (txins, mut input_txuos) = 
            input_selection(
                None, 
                &mut needed_value, 
                &input_txuos, 
                gtxd.clone().get_collateral(),
                None,//Some(native_script_address).as_ref(),
        )?;

        
        /*
        if let Some(signer) = minttxd.get_signer() {
            if input_txuos.contains_any(&signers_address_utxos.0) {
                info!("\n\nUtxo Input set contains minimum one utxo from the listing address\n\n");
            } else {
                if !signers_address_utxos.0.is_empty() {
                    // ToDo:
                    // In this case it would be better to have some larger Ada only Utxo -> Create a function to find one
                    txins.add(&signers_address_utxos.0.get(0).input());
                    input_txuos.add(&signers_address_utxos.0.get(0));
                } else {
                    return Err(
                        MurinError::new(
                            &format!(
                                "Error: The Address which is needed for signature does not contain ADA, please provide suitable amount of ADA to: {:?}",signer.to_bech32(None)
                            )
                        )
                    )
                }
            }
        }
         */
        let saved_input_txuos = input_txuos.clone();
        info!("Saved Inputs: {:?}",saved_input_txuos);

        let vkey_counter = get_vkey_count(&input_txuos, collateral_input_txuo.as_ref()) + 1; // +1 dues to signature in finalize
        debug!("\n\n\n\n\nTxIns Before Balance:\n {:?}\n\n\n\n\n",input_txuos);

        // ToDo:
        let mut mint_val_zero_coin = mint_val.clone();
        mint_val_zero_coin.set_coin(&cutils::to_bignum(0u64));

        let txouts_fin = balance_tx(
            &mut input_txuos,             
            &minttokens,         
            &mut txouts,
            Some(mint_val_zero_coin).as_ref(), // but not the ADA!!!!                   
            fee,                           
            &mut fee_paied,                
            &mut first_run,                
            &mut txos_paied,
            &mut tbb_values,
            &change_address, //who is sender ? 
            &change_address,         
            &mut acc,
            None,
            &dummy
            )?;

            
        ////////////////////////////////////////////////////////////////////////////////////////////
        // 
        // MINT ASSETS 
        // 
        ////////////////////////////////////////////////////////////////////////////////////////////
        let mut mintasset = clib::MintAssets::new();
        
        for token in minttokens {
            mintasset.insert(&token.1, clib::utils::Int::new(&token.2));
        }

        let mint = clib::Mint::new_from_entry(&mintpolicy, &mintasset);


        let slot = cutils::to_bignum(gtxd.clone().get_current_slot() + get_ttl_tx(&gtxd.clone().get_network()));
        let mut txbody = clib::TransactionBody::new_tx_body(&txins,&txouts_fin,&fee);
        txbody.set_ttl(&slot);
        info!("\nTxOutputs: {:?}\n",txbody.outputs());
        debug!("\nTxInputs: {:?}\n",txbody.inputs());

        txbody.set_auxiliary_data_hash(&aux_data_hash);

        txbody.set_mint(&mint);

        
         // Set network Id
        if gtxd.get_network() == clib::NetworkIdKind::Testnet {
            txbody.set_network_id(&clib::NetworkId::testnet());
        }else{
            txbody.set_network_id(&clib::NetworkId::mainnet());
        }  

        //let req_signer = native_script.get_required_signers();
        //info!("Len Req SIgner: {:?}",req_signer.len());
        //for i in 0..req_signer.len() {
        //    info!("Required Signer: {:?}" ,req_signer.get(i).to_bech32("pkh_")) //req_signer.len()
        //}

        let mut txwitness = clib::TransactionWitnessSet::new();
        let mut native_scripts = clib::NativeScripts::new();
        native_scripts.add(&native_script);
        txwitness.set_native_scripts(&native_scripts);

        let root_key1 = clib::crypto::Bip32PrivateKey::from_bytes(&hex::decode(&pvks[0])?)?;
        let account_key1 = root_key1.derive(harden(1852u32)).derive(harden(1815u32)).derive(harden(0u32));
        let prv1 = account_key1.to_raw_key(); // for signatures
        let vkwitness_1d1 =cutils::make_vkey_witness(&cutils::hash_transaction(&txbody), &prv1);
        

        let mut vkeywitnesses = ccrypto::Vkeywitnesses::new();
        vkeywitnesses.add(&vkwitness_1d1);
        txwitness.set_vkeys(&vkeywitnesses);
        debug!("TxWitness: {:?}", hex::encode(txwitness.to_bytes()));

        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        debug!("Vkey Counter at End: {:?}",vkey_counter);
        Ok((txbody,txwitness,aux_data,saved_input_txuos,vkey_counter))
}

pub async fn build_oneshot_mint_multisig (
            gtxd : &TxData, 
            minttxd : &MinterTxData, 
            pvks : &Vec::<String>,
            ns_script : &clib::NativeScript, 
            liquidity_addr          : &Address,
        ) -> std::result::Result<crate::htypes::BuildOutput, MurinError> {
            // Temp until Protocol Parameters fixed
            let mem = cutils::to_bignum(7000000u64);        //cutils::to_bignum(7000000u64);    
            let steps = cutils::to_bignum(2500000000u64);   //cutils::to_bignum(3000000000u64);
            let ex_unit_price : crate::htypes::ExUnitPrice = crate::ExUnitPrice { priceSteps: 7.21e-5, priceMemory:  5.77e-2 };
            let a = cutils::to_bignum(44u64);
            let b = cutils::to_bignum(155381u64);

            //Create Tx
            let (txbody_,mut txwitness_, aux_data_, _,vkey_counter) = 
                perform_mint(&cutils::to_bignum(2000000), ns_script, gtxd, minttxd, pvks, liquidity_addr, true)?;

            let dummy_vkeywitnesses = make_dummy_vkeywitnesses(vkey_counter); 
            txwitness_.set_vkeys(&dummy_vkeywitnesses);

            // Build and encode dummy transaction
            let transaction_ = clib::Transaction::new(&txbody_,&txwitness_,Some(aux_data_.clone()));

            let calculated_fee = calc_txfee(&transaction_,&a,&b,ex_unit_price.clone(),&steps,&mem,true);   
            let (txbody, txwitness, aux_data,used_utxos,vkey_counter_2) = 
                perform_mint(&calculated_fee, ns_script, gtxd, minttxd, &pvks, liquidity_addr, false)?;          

            let transaction2 = clib::Transaction::new(&txbody,&txwitness_,Some(aux_data.clone()));


            if vkey_counter_2 != vkey_counter || transaction2.to_bytes().len() != transaction_.to_bytes().len() {
                let dummy_vkeywitnesses = make_dummy_vkeywitnesses(vkey_counter_2); 
                txwitness_.set_vkeys(&dummy_vkeywitnesses);

                let calculated_fee = calc_txfee(&transaction2,&a,&b,ex_unit_price,&steps,&mem,true);   
                let (txbody, txwitness, aux_data,used_utxos,_) = 
                    perform_mint(&calculated_fee, ns_script, gtxd, minttxd, &pvks, liquidity_addr, false)?;     
                info!("Fee: {:?}",calculated_fee);
                debug!("\n\n\nDummy vkey_counter: {:?} \n\n",vkey_counter);
                debug!("\n\n\nDummy vkey_counter_2: {:?} \n\n",vkey_counter_2);
                Ok(tx_output_data (txbody,txwitness,aux_data,used_utxos.to_hex()?,0u64,false)?)
            } else {
            info!("Fee: {:?}",calculated_fee);
                Ok(tx_output_data(txbody,txwitness,aux_data,used_utxos.to_hex()?,0u64,false)?)
            }            

        }