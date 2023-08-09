use crate::datamodel::Operation;
use crate::protocol::create_response;
use crate::BuildStdTx;
use crate::CmdError;

use murin::clib::address::Address;
use murin::TransactionUnspentOutputs;

use murin::modules::transfer::models::{TransWallet, TransWallets};
use murin::stdtx::build_wallet_asset_transfer::{AtSATBuilder, AtSATParams};

use murin::{b_decode_addr, PerformTxb};

// Handler for ordinary token transfers
pub(crate) async fn handle_stx(bss: &BuildStdTx) -> crate::Result<String> {
    match bss
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?
    {
        Operation::StdTx {
            wallet_addresses,
            transfers,
        } => {
            let err = Err(CmdError::Custom {
                str: format!(
                    "ERROR wrong data provided for script specific parameters: '{:?}'",
                    bss.transaction_pattern().operation()
                ),
            }
            .into());
            if transfers.is_empty() {
                return err;
            }

            if let Some(addresses) = wallet_addresses {
                for addr in addresses.iter() {
                    b_decode_addr(addr).await?;
                }
            }
        }
        _ => {
            return Err(CmdError::Custom {
                str: format!("ERROR wrong data provided for '{:?}'", bss.tx_type()),
            }
            .into());
        }
    }
    log::debug!("Checks okay...");

    let mut bsstp = bss.transaction_pattern().clone();

    log::debug!("Try to create raw data...");
    let std_asset_txd = bss
        .transaction_pattern()
        .operation()
        .unwrap()
        .into_stdassettx()
        .await?;

    let addresses =
        std_asset_txd
            .wallet_addresses
            .iter()
            .fold(Vec::<String>::new(), |mut acc, n| {
                acc.push(n.to_bech32(None).unwrap());
                acc
            });
    bsstp.set_used_addresses(&addresses);

    log::debug!("Try to create raw data2...");
    let mut gtxd = bsstp.into_txdata().await?;
    log::debug!("Try to create raw data3...");
    if !std_asset_txd.wallet_addresses.is_empty() {
        let wallet_utxos = std_asset_txd.wallet_addresses.iter().fold(
            TransactionUnspentOutputs::new(),
            |mut acc, n| {
                acc.merge(mimir::get_address_utxos(&n.to_bech32(None).unwrap()).unwrap());
                acc
            },
        );
        gtxd.set_inputs(wallet_utxos);

        // ToDo: go through all addresses and check all stake keys are equal
        let sa = murin::get_reward_address(&std_asset_txd.wallet_addresses[0])?;
        gtxd.set_stake_address(sa);
        gtxd.set_senders_addresses(std_asset_txd.wallet_addresses.clone());
    }

    log::debug!("Try to determine slot...");
    let mut dbsync = match mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()),
            }
            .into());
        }
    };
    let slot = match mimir::get_slot(&mut dbsync) {
        Ok(s) => s,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!(
                    "ERROR could not determine current slot: '{:?}'",
                    e.to_string()
                ),
            }
            .into());
        }
    };
    gtxd.set_current_slot(slot as u64);

    let mut wallets = TransWallets::new();

    let first_address_str =
        mimir::select_addr_of_first_transaction(&gtxd.get_stake_address().to_bech32(None)?)?;
    let first_addr = Address::from_bech32(&first_address_str)?;

    // ToDo:
    // - Add Wallets

    // If addresses are provided check they all belong to the same wallet, if yes
    // get utxos for the addresses and build TransWallet with that UTxOs

    let uw = TransWallet::new(&first_addr, &gtxd.get_inputs());
    wallets.add_wallet(&uw);

    // - Add Endpoint to get AssetHandles From AddressSet

    let txb_param: AtSATParams = (&std_asset_txd, &wallets, &first_addr);
    let asset_transfer = AtSATBuilder::new(txb_param);
    let builder = murin::TxBuilder::new(&gtxd, &vec![]);
    let bld_tx = builder.build(&asset_transfer).await;

    if let Err(err) = &bld_tx {
        return Err(CmdError::Custom {
            str: err.to_string(),
        }
        .into());
    }
    let bld_tx = bld_tx?;

    log::debug!("Try to create raw tx...");
    let tx = murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &std_asset_txd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bss.customer_id()),
        &[],
    );
    debug!("RAWTX data: {:?}", tx);

    log::debug!("Try to create response data...");
    let ret = create_response(
        &bld_tx,
        &tx,
        bss.transaction_pattern().wallet_type().as_ref(),
    )?;
    Ok(serde_json::json!(ret).to_string())
}

#[cfg(test)]
mod tests {
    use crate::FinalizeStdTx;
    use crate::datamodel::models::StdTxType;
    use crate::BuildStdTx;
    use crate::datamodel::TransactionPattern;
    use crate::Operation;
    use murin::{
        clib::address::Address,
        MurinError, 
        txbuilders::{
            stdtx::{
                StandardTxData,
                build_wallet_asset_transfer::AtSATBuilder,
            },
            modules::transfer::models::TransWallets,
            PerformTxb,
        },
    };
    use std::str::FromStr;
    use std::env::set_var;
    use murin::modules::transfer::models::TransWallet;

    #[tokio::test]
    async fn test2() -> Result<(), MurinError> {
        // // hugin env
        // set_var("ADM_USER", "trsfasfue");
        // set_var("POW", "trsfasfue");
        // set_var("ODIN_URL", "trsfasfue");
        // set_var("PLATFORM_DB_URL", "trsfasfue");
        // set_var("MOUNT", "trsfasfue");
        // set_var("VPATH", "trsfasfue");

        // // dvltath
        // set_var("VSOCKET_PATH", "trsfasfue");
        // set_var("OROLE_ID", "trsfasfue");
        // set_var("OSECRET_ID", "trsfasfue");
        // set_var("RUST_LOG", "trsfasfue");
        // set_var("VAULT_TOKEN", "trsfasfue");

        // // gungnir env
        // set_var("REWARDS_DB_URL", "trsfasfue");

        // // mimir env
        // set_var("DBSYNC_DB_URL", "trsfasfue"); // needed for live data, but not for test data

        // // murin env
        // set_var("CARDANO_CLI_PATH", "trsfasfue");
        // set_var("CARDANO_PROTOCOL_PARAMETER_PATH", "trsfasfue");
        // set_var("TX_SUBMIT_ENDPOINT1", "trsfasfue"); // needed if you wanna submit
        // set_var("TX_SUBMIT_ENDPOINT2", "trsfasfue"); // needed if you wanna submit
        // set_var("TX_SUBMIT_ENDPOINT3", "trsfasfue"); // needed if you wanna submit

        set_var("REDIS_DB", "redis://127.0.0.1:6379/0"); // required env
        set_var("REDIS_DB_URL_UTXOMIND","redis://127.0.0.1:6379/0"); // required env
        set_var("REDIS_CLUSTER", "false"); // required env

        // set_var("TXGSET", "tfsafasrue");
        set_var("USED_UTXO_DATASTORE_1", "UTXOSTORE1"); // needed
        set_var("USED_UTXO_DATASTORE_2", "UTXOSTORE2"); // needed
        set_var("USED_UTXO_DATASTORE_3", "UTXOSTORE3"); // needed
        // set_var("PENDING_TX_DATASTORE_1", "sfsafa");
        // set_var("PENDING_TX_DATASTORE_2", "fasfsaf");
        // set_var("PENDING_TX_DATASTORE_3", "fsafasf");

        // // sleipnir env
        // set_var("JWT_KEY", "trsfasfue");
        // set_var("DRASIL_REWARD_DB", "trsfasfue");

        // // frigg env
        // set_var("JWT_PUB_KEY", "trsfasfue");
        // set_var("RUST_LOG", "trsfasfue");
        // set_var("POD_HOST", "trsfasfue");
        // set_var("POD_PORT", "trsfasfue");
        // set_var("VERIFICATION_LINK", "trsfasfue");
        // set_var("SMTP_USER", "trsfasfue");
        // set_var("SMTP_PW", "trsfasfue");
        // set_var("FROM_EMAIL", "trsfasfue");
        // set_var("EMAIL_API_KEY", "trsfasfue");
        // set_var("AMQP_ADDR", "trsfasfue");
        // set_var("QUEUE_NAME", "trsfasfue");
        // set_var("CONSUMER_NAME", "trsfasfue");

        // // heimdallr env
        // set_var("JWT_PUB_KEY", "trsfasfue");
        // set_var("ODIN_URL", "trsfasfue");

        // // loki env
        // set_var("AMWP_ADDR", "trsfasfue");

        // // geri env
        // set_var("STREAM_TRIMMER", "trsfasfue");
        // set_var("STREAMS", "trsfasfue");
        // set_var("TIMEOUT", "trsfasfue");

        // data preparation
        let customer_id = 10;
        let txtype: StdTxType = StdTxType::StandardTx;
        let tx_id = "9e24114313ae441c1b68125a0cef284c141a3f6ef270fc5608e255424a3c3219".to_string();
        let signature = "100818258204949628654d1fabf39d007ecd9c7ab92df8b1ed349a1d3dd57da62390d378e03
            5840dbabd6d0cfb4d01b1986f98dde29e64dbda251a9887d68272d417f0dab4410cf313d3578aa8182fa5f0b1310c7ca
            d2be27e34c1bc7465310fa44a6112ede7d05f5d90103a100a1190539a269636f6d706c6574656400646e616d656b6865
            6c6c6f20776f726c64".to_string();

        let std_asset_txd: StandardTxData = StandardTxData::from_str("{
            \"wallet_addresses\": [
                \"addr_test1qqt86eq9972q3qttj6ztje97llasktzfzvhmdccqjlqjaq2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qy6q5t2\",
                \"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"addr_test1qqdp3cry5vc2gfjljctdu638tvkcqfx40fjunht9hrmru5zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qnaxxgs\",
                \"addr_test1qr2mw080ujz0unmpn9lx5ftfuewc6htyr6v3a0svul2zgezcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qgryf7t\",
                \"addr_test1qr7tqh7tsg4lut3jv6tsfwlv464m6knjjw90ugyz8uzgr6zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qt0jxzj\",
                \"addr_test1qrscurjp292sxv24sepj7ghq4ydkkekzaz53zwfswcna6ljcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6q8pu3l5\",
                \"addr_test1qqssrphse6qmp9h0ksu5vfmsx99tfl2lc6rhvy2spd5wr86cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw59j4j\",
                \"addr_test1qqgagc0fy6nm0qe4h8zqxsg952tqjeg7l7j0agd0cx4u25zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qxvept2\",
                \"addr_test1qrjmru0chcxw0q7y099k2elzr45sh77gafkzj75xqwe66zzcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qthp2py\",
                \"addr_test1qq78tygxpu7a53rz7m6jnrtf5s8sc6dvg63jz80uyqrfswzcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qhe9ekw\",
                \"addr_test1qpvntdhn6s9d09z72f75atv8ha8qax46a5tfpcf7cp2jwm6cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6ql87t33\",
                \"addr_test1qqrja5l2hdl5gdyz7xvm948jg7vc9ed0uzp28yqgveaxww6cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qkw5xzz\",
                \"addr_test1qpzmsl9qfyzlh94049ya2ffjy8akvhmrhc6azdccmdyn2j2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qzudgs9\"
            ],
            \"transfers\":[
            {
                \"receiver\":\"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : [\"Hello My friend this is for you\"]
            }
        ]
        }")?;
        // wallet_addresses: Vec<Address>,
        // transfers: Vec<AssetTransfer>,

        let mut wallets: TransWallets = TransWallets::new();

        let inputs: murin::TransactionUnspentOutputs = murin::TransactionUnspentOutputs::new();
        let network: murin::NetworkIdKind = murin::NetworkIdKind::Testnet;

        let first_addr = Address::from_bech32("addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu")?;

        let gtxd: murin::TxData = murin::TxData::new(
            None,
            vec![
                first_addr.clone()
            ],
            None,
            inputs,
            network,
            0,
        )?;

        let uw = TransWallet::new(&first_addr, &gtxd.get_inputs());
        wallets.add_wallet(&uw);

        let script_spec: Operation = Operation::StdTx{
            transfers: vec![],
            wallet_addresses: None,
        };

        let txpattern: TransactionPattern = TransactionPattern::new_empty(
            customer_id,
            &script_spec,
            0,
        );

        let bss: BuildStdTx = BuildStdTx::new(
            customer_id, 
            txtype.clone(),
            txpattern,
        );

        // build tx
        let txb_param: (&StandardTxData, &TransWallets, &Address) = (&std_asset_txd, &wallets, &first_addr);
        let asset_transfer = AtSATBuilder::new(txb_param);
        let builder = murin::TxBuilder::new(&gtxd, &vec![]);
        log::debug!("Try to create raw tx...");
        let bld_tx = builder.build(&asset_transfer).await?;
        let raw_tx = murin::utxomngr::RawTx::new(
            &bld_tx.get_tx_body(),
            &bld_tx.get_txwitness(),
            &bld_tx.get_tx_unsigned(),
            &bld_tx.get_metadata(),
            &gtxd.to_string(),
            &std_asset_txd.to_string(),
            &bld_tx.get_used_utxos(),
            &hex::encode(gtxd.get_stake_address().to_bytes()),
            &(bss.customer_id()),
            &[],
        );
        debug!("RAWTX data: {:?}", raw_tx);
        
        debug!("Try to store raw tx...");
        let tx_id = murin::utxomngr::txmind::store_raw_tx(&raw_tx)?;

        let finalize_std_tx: FinalizeStdTx = FinalizeStdTx::new(
            customer_id,
            txtype,
            tx_id,
            signature,
        );

        let raw_tx = murin::utxomngr::txmind::read_raw_tx(&finalize_std_tx.get_tx_id())?;
        // let raw_tx = murin::utxomngr::txmind::read_raw_tx(&tx_id)?;
        let standard_tx_data: StandardTxData = murin::stdtx::StandardTxData::from_str(raw_tx.get_tx_specific_rawdata())?;

        let trans_wallets: TransWallets = TransWallets::new();
        let address: Address = Address::from_hex("")?;
        let atsat_params: (&StandardTxData, &TransWallets, &Address) = (
            &standard_tx_data, 
            &trans_wallets, 
            &address
        );
        let _atsat_builder = AtSATBuilder::new(atsat_params);
        Ok(())
    }
}