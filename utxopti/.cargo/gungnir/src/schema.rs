/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
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
        mode -> crate::Calculationmode,
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
    }
}

table! {
    wlalloc (wl,addr) {
        wl -> Int8,
        addr -> Int8,
    }
}

table! {
    whitelist (id) {
        id -> Int8,
        max_addr_repeat -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    mint_projects (id) {
        id  -> Int8,
        customer_name -> Varchar,
        project_name -> Varchar,
        user_id -> Int8,
        contract_id -> Int8,
        whitelist_id -> Nullable<Int8>,
        mint_start_date -> Timestamptz,
        mint_end_date -> Nullable<Timestamptz>,
        storage_folder -> Varchar,
        max_trait_count -> Int4,
        collection_name -> Varchar,
        author -> Varchar,
        meta_description -> Varchar,
        max_mint_p_addr -> Nullable<Int4>,
        reward_minter -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    nft_table (project_id,asset_name_b){
        project_id -> Int8,
        asset_name_b -> Bytea,
        asset_name -> Varchar,
        picture_id -> Varchar,
        file_name -> Varchar,
        ipfs_hash -> Nullable<Varchar>,
        trait_category -> Array<Text>,
        traits -> Array<Array<Text>>,
        metadata -> Text,
        payment_addr -> Nullable<Varchar>,
        minted -> Bool,
        tx_hash -> Nullable<Varchar>,
        confirmed -> Bool,
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
);
