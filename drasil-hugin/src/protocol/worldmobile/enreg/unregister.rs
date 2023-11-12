/*
use super::RegistrationDatum;
use crate::{
    models::{
        BuilderResult, EarthNodeConfig, RegisterEarthNode, TransactionSchema, UnsignedTransaction,
    },
    modules::txprocessor::{
        error::TransactionBuildingError,
        protocolparams::{
            hash::{self, blake2b256},
            self_plutus::Languages,
        },
        transactions::smartcontract::enregistration::{
            restore_wmreg_datum, RegistrationRedeemer, ENREGCONTRACT,
        },
    },
};
use cardano_serialization_lib::{
    address::{Address, BaseAddress, EnterpriseAddress, StakeCredential},
    crypto::{Ed25519KeyHash, ScriptDataHash, ScriptHash},
    fees::LinearFee,
    plutus::{
        self, ExUnitPrices, ExUnits, PlutusData, PlutusList, PlutusScript, Redeemer, RedeemerTag,
    },
    tx_builder::{
        tx_inputs_builder::{
            DatumSource, PlutusScriptSource, PlutusWitness, PlutusWitnesses, TxInputsBuilder,
        },
        CoinSelectionStrategyCIP2, TransactionBuilder, TransactionBuilderConfigBuilder,
    },
    utils::{hash_plutus_data, to_bignum, Value},
    AssetName, Assets, MultiAsset, TransactionInputs, TransactionOutput, UnitInterval,
};
use cdp::provider::CardanoDataProvider;
use dcslc::{
    calc_min_ada_for_utxo, decode_transaction_unspent_outputs, extract_assets,
    find_utxos_by_address, get_pubkeyhash, get_stakeaddr_from_addr, make_fingerprint, Token,
    TransactionUnspentOutput,
};
use log::debug;
use sha2::Digest;


pub(crate) async fn handle_unregister_earth_node(
    tx_schema: TransactionSchema,
) -> Result<BuilderResult, TransactionBuildingError> {
    let op_data = match tx_schema.operation {
        Some(d) => serde_json::from_value::<RegisterEarthNode>(d)?,
        None => return Err(TransactionBuildingError::StandardTransactionBuildingError),
    };

    let policy_str =
        std::env::var("ENNFT_POLICY").expect("No ENNFT policyID set for this tx-building service");
    let policy = ScriptHash::from_bytes(hex::decode(&policy_str)?)?;

    let smartcontract = PlutusScript::from_bytes_v2(hex::decode(ENREGCONTRACT.as_str())?)?;
    let scripthash = smartcontract.hash();
    let cred = StakeCredential::from_scripthash(&scripthash);

    // check against wallet
    let utxos = decode_transaction_unspent_outputs(
        &tx_schema.utxos.unwrap(),
        tx_schema.collateral.as_ref(),
        tx_schema.excludes.as_ref(),
    )?;

    let collateral = decode_transaction_unspent_outputs(
        tx_schema
            .collateral
            .as_ref()
            .expect("no collateral utxos provided"),
        None,
        None,
    )?;

    // check against dataprovider
    let dp = cdp::DataProvider::new(cdp::DBSyncProvider::new(cdp::Config {
        db_path: std::env::var("DBSYNC_URL").unwrap(),
    }));

    let stake_address = if let Some(stake_addr) = tx_schema.stake_address {
        dcslc::addr_from_str(&stake_addr[0].clone())?
            .to_bech32(None)
            .unwrap()
    } else {
        return Err(TransactionBuildingError::RewardAddressNotFound);
    };

    debug!("Stake Address: {}", stake_address);
    debug!("Wallet UTxOs empty: {:?}\n", utxos.is_empty());
    let first_address = dp.first_transaction_from_stake_addr(&stake_address).await?;
    let first_pkh = BaseAddress::from_address(&first_address)
        .unwrap()
        .payment_cred()
        .to_keyhash()
        .unwrap();
    log::debug!(
        "\nPubKeyHash First Address: {}\n",
        hex::encode(first_pkh.to_bytes())
    );
    let script_address = EnterpriseAddress::new(first_address.network_id()?, &cred).to_address();
    log::debug!("\nScript Address: {}\n", script_address.to_bech32(None)?);

    let sutxos = dp
        .script_utxos(&script_address.to_bech32(None)?)
        .await
        .unwrap();

    let ennft_tokeninfo = dp.token_info(&op_data.ennft_assetname).await?;
    let script_utxos = sutxos.find_utxos_containing_asset(
        &policy,
        &AssetName::new(hex::decode(&ennft_tokeninfo.tokenname)?)?,
    )?;

    let pubkeyhash = get_pubkeyhash(&first_address)?;

    if script_utxos.len() != 1 {
        return Err(TransactionBuildingError::Custom(
            "smart contract does not contain the specified ENNFT".to_owned(),
        ));
    }
    // TODO: UNCOMMENT
    assert_eq!(ennft_tokeninfo.policy, policy_str);
    let script_utxo = script_utxos.get(0);
    log::debug!("Try to restore datum...");
    if script_utxo.output().plutus_data().is_none() {
        return Err(TransactionBuildingError::Custom(
            "the utxo of the ENNFT does not contain a datum".to_owned(),
        ));
    }

    let datum = script_utxo.output().plutus_data().unwrap();
    let onchain_registration_datum = restore_wmreg_datum(&datum.to_bytes())?;

    log::debug!(
        "\nRestored Inline Datum: {:?}\n",
        &onchain_registration_datum
    );
    // Registration Datum
    let datum_in_request = RegistrationDatum {
        enOperatorAddress: op_data.config.operator_address.as_bytes().to_vec(),
        enConsensusPubkey: op_data.config.consensus_pub_key.as_bytes().to_vec(),
        enMerkleTreeRoot: op_data.config.merkle_tree_root.as_bytes().to_vec(),
        enCceAddress: op_data.config.cce_address.as_bytes().to_vec(),
        enUsedNftTn: AssetName::new(hex::decode(&ennft_tokeninfo.tokenname)?)?,
        enOwner: pubkeyhash.clone(),
    };
    log::debug!("\nBuilt Datum: {:?}\n", &datum_in_request);
    assert_eq!(
        onchain_registration_datum, datum_in_request,
        "send and restored datums: {onchain_registration_datum:?} and \n{datum_in_request:?}"
    );
    Err(())
}
 */