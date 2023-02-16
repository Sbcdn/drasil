/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "Calculationmode"))]
    pub struct Calculationmode;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "whitelisttype"))]
    pub struct WhitelistType;
}

table! {
    airdrop_parameter (id) {
        id -> Int8,
        contract_id -> Int8,
        user_id -> Int8,
        airdrop_token_type -> Varchar,
        distribution_type -> Varchar,
        selection_type -> Text,
        args_1 -> Array<Text>,
        args_2 -> Array<Text>,
        args_3 -> Array<Text>,
        whitelist_ids -> Nullable<Array<Int8>>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    airdrop_whitelist (id) {
        id -> Int8,             // -> whitelist-id
        contract_id -> Int8,
        user_id -> Int8,
        reward_created -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    claimed (id) {
        id -> Int8,
        stake_addr -> Varchar,
        payment_addr -> Varchar,
        fingerprint -> Varchar,
        amount -> Numeric,
        contract_id -> Int8,
        user_id -> Int8,
        txhash -> Varchar,
        invalid -> Nullable<Bool>,
        invalid_descr -> Nullable<Text>,
        timestamp -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    rewards (id) {
        id -> Int8,
        stake_addr -> Varchar,
        payment_addr -> Varchar,
        fingerprint -> Varchar,
        contract_id -> Int8,
        user_id -> Int8,
        tot_earned -> Numeric,
        tot_claimed -> Numeric,
        oneshot -> Bool,
        last_calc_epoch -> Int8,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {

    token_whitelist (id) {
        id -> Int8,
        fingerprint -> Nullable<Varchar>,
        policy_id -> Varchar,
        tokenname -> Nullable<Varchar>,
        contract_id -> Int8,
        user_id -> Int8,
        vesting_period -> Timestamptz,
        pools -> Array<Text>,
        mode -> crate::schema::sql_types::Calculationmode,
        equation -> Text,
        start_epoch -> Int8,
        end_epoch -> Nullable<Int8>,
        modificator_equ -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    wladdresses (id) {
        id -> Int8,
        payment_address -> Varchar,
        stake_address -> Nullable<Varchar>,
    }
}

table! {
    wlalloc (wl,addr) {
        wl -> Int8,
        addr -> Int8,
        specific_asset -> Nullable<Jsonb>,
    }
}

table! {
    whitelist (id) {
        id -> Int8,
        user_id -> Int8,
        max_addr_repeat -> Int4,
        wl_type -> crate::schema::sql_types::WhitelistType,
        description -> Text,
        notes -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    mint_projects (id) {
        id  -> Int8,
        project_name -> Varchar,
        user_id -> Int8,
        mint_contract_id -> Int8,
        whitelists -> Nullable<Array<Int8>>,
        mint_start_date -> Timestamptz,
        mint_end_date -> Nullable<Timestamptz>,
        storage_type -> Varchar,
        storage_url -> Nullable<Varchar>,
        storage_access_token -> Nullable<Varchar>,
        collection_name -> Varchar,
        author -> Varchar,
        meta_description -> Varchar,
        meta_common_nft_name -> Nullable<Varchar>,
        max_mint_p_addr -> Nullable<Int4>,
        nft_table_name -> Varchar,
        active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    nft_table (project_id,asset_name_b){
        project_id -> Int8,
        asset_name_b -> Bytea,
        asset_name -> Varchar,
        fingerprint -> Varchar,
        nft_id -> Varchar,
        file_name -> Nullable<Varchar>,
        ipfs_hash -> Nullable<Varchar>,
        metadata -> Nullable<Text>,
        claim_addr -> Nullable<Varchar>,
        minted -> Bool,
        tx_hash -> Nullable<Varchar>,
        confirmed -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    mint_rewards (id){
        id -> Int8,
        project_id -> Int8,
        pay_addr -> Varchar,
        nft_ids -> Array<Bytea>,
        v_nfts_b -> Array<Bytea>,
        processed -> Bool,
        minted -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    discount (id){
        id -> Int8,
        contract_id -> Int8,
        user_id -> Int8,
        policy_id -> Varchar,
        fingerprint -> Nullable<Varchar>,
        metadata_path -> Array<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

allow_tables_to_appear_in_same_query!(
    airdrop_parameter,
    airdrop_whitelist,
    claimed,
    rewards,
    token_whitelist,
    wladdresses,
    wlalloc,
    whitelist,
    mint_projects,
    nft_table,
    mint_rewards,
);
