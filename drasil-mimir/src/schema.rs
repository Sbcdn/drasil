pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "Syncstatetype"))]
    pub struct Syncstatetype;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "Scriptpurposetype"))]
    pub struct Scriptpurposetype;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "Rewardtype"))]
    pub struct Rewardtype;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "Scripttype"))]
    pub struct Scripttype;
}

table! {
    unspent_utxos (id){
        id -> Int8,
        tx_id -> Int8,
        hash -> Bytea,
        index -> Int2,
        address -> Varchar,
        value -> Numeric,
        data_hash -> Nullable<Bytea>,
        address_has_script -> Bool,
        stake_address -> Nullable<Varchar>,
    }
}
table! {
    ada_pots (id) {
        id -> Int8,
        slot_no -> Int4,
        epoch_no -> Int4,
        treasury -> Numeric,
        reserves -> Numeric,
        rewards -> Numeric,
        utxo -> Numeric,
        deposits -> Numeric,
        fees -> Numeric,
        block_id -> Int8,
    }
}

table! {
    admin_user (id) {
        id -> Int8,
        username -> Varchar,
        password -> Varchar,
    }
}

table! {
    block (id) {
        id -> Int8,
        hash -> Bytea,
        epoch_no -> Nullable<Int4>,
        slot_no -> Nullable<Int8>,
        epoch_slot_no -> Nullable<Int4>,
        block_no -> Nullable<Int4>,
        previous_id -> Nullable<Int8>,
        slot_leader_id -> Int8,
        size -> Int4,
        time -> Timestamp,
        tx_count -> Int8,
        proto_major -> Int4,
        proto_minor -> Int4,
        vrf_key -> Nullable<Varchar>,
        op_cert -> Nullable<Bytea>,
        op_cert_counter -> Nullable<Int8>,
    }
}

table! {
    collateral_tx_in (id) {
        id -> Int8,
        tx_in_id -> Int8,
        tx_out_id -> Int8,
        tx_out_index -> Int2,
    }
}

table! {
    cost_model (id) {
        id -> Int8,
        costs -> Jsonb,
        block_id -> Int8,
    }
}

table! {
    datum (id) {
        id -> Int8,
        hash -> Bytea,
        tx_id -> Int8,
        value -> Nullable<Jsonb>,
    }
}

table! {
    delegation (id) {
        id -> Int8,
        addr_id -> Int8,
        cert_index -> Int4,
        pool_hash_id -> Int8,
        active_epoch_no -> Int8,
        tx_id -> Int8,
        slot_no -> Int4,
        redeemer_id -> Nullable<Int8>,
    }
}

table! {
    delisted_pool (id) {
        id -> Int8,
        hash_raw -> Bytea,
    }
}

table! {
    epoch (id) {
        id -> Int8,
        out_sum -> Numeric,
        fees -> Numeric,
        tx_count -> Int4,
        blk_count -> Int4,
        no -> Int4,
        start_time -> Timestamp,
        end_time -> Timestamp,
    }
}

table! {
    epoch_param (id) {
        id -> Int8,
        epoch_no -> Int4,
        min_fee_a -> Int4,
        min_fee_b -> Int4,
        max_block_size -> Int4,
        max_tx_size -> Int4,
        max_bh_size -> Int4,
        key_deposit -> Numeric,
        pool_deposit -> Numeric,
        max_epoch -> Int4,
        optimal_pool_count -> Int4,
        influence -> Float8,
        monetary_expand_rate -> Float8,
        treasury_growth_rate -> Float8,
        decentralisation -> Float8,
        entropy -> Nullable<Bytea>,
        protocol_major -> Int4,
        protocol_minor -> Int4,
        min_utxo_value -> Numeric,
        min_pool_cost -> Numeric,
        nonce -> Nullable<Bytea>,
        coins_per_utxo_word -> Nullable<Numeric>,
        cost_model_id -> Nullable<Int8>,
        price_mem -> Nullable<Float8>,
        price_step -> Nullable<Float8>,
        max_tx_ex_mem -> Nullable<Numeric>,
        max_tx_ex_steps -> Nullable<Numeric>,
        max_block_ex_mem -> Nullable<Numeric>,
        max_block_ex_steps -> Nullable<Numeric>,
        max_val_size -> Nullable<Numeric>,
        collateral_percent -> Nullable<Int4>,
        max_collateral_inputs -> Nullable<Int4>,
        block_id -> Int8,
    }
}

table! {
    epoch_reward_total_received (id) {
        id -> Int8,
        earned_epoch -> Int4,
        amount -> Numeric,
    }
}

table! {
    epoch_stake (id) {
        id -> Int8,
        addr_id -> Int8,
        pool_id -> Int8,
        amount -> Numeric,
        epoch_no -> Int4,
    }
}

table! {
    epoch_sync_time (id) {
        id -> Int8,
        no -> Int8,
        seconds -> Int8,
        state -> crate::schema::sql_types::Syncstatetype,
    }
}

table! {
    ma_tx_mint (id) {
        id -> Int8,
        quantity -> Numeric,
        tx_id -> Int8,
        ident -> Int8,
    }
}

table! {
    ma_tx_out (id) {
        id -> Int8,
        quantity -> Numeric,
        tx_out_id -> Int8,
        ident -> Int8,
    }
}

table! {
    meta (id) {
        id -> Int8,
        start_time -> Timestamp,
        network_name -> Varchar,
        version -> Varchar,
    }
}

table! {
    multi_asset (id) {
        id -> Int8,
        policy -> Bytea,
        name -> Bytea,
        fingerprint -> Varchar,
    }
}

table! {
    param_proposal (id) {
        id -> Int8,
        epoch_no -> Int4,
        key -> Bytea,
        min_fee_a -> Nullable<Numeric>,
        min_fee_b -> Nullable<Numeric>,
        max_block_size -> Nullable<Numeric>,
        max_tx_size -> Nullable<Numeric>,
        max_bh_size -> Nullable<Numeric>,
        key_deposit -> Nullable<Numeric>,
        pool_deposit -> Nullable<Numeric>,
        max_epoch -> Nullable<Numeric>,
        optimal_pool_count -> Nullable<Numeric>,
        influence -> Nullable<Float8>,
        monetary_expand_rate -> Nullable<Float8>,
        treasury_growth_rate -> Nullable<Float8>,
        decentralisation -> Nullable<Float8>,
        entropy -> Nullable<Bytea>,
        protocol_major -> Nullable<Int4>,
        protocol_minor -> Nullable<Int4>,
        min_utxo_value -> Nullable<Numeric>,
        min_pool_cost -> Nullable<Numeric>,
        coins_per_utxo_word -> Nullable<Numeric>,
        cost_model_id -> Nullable<Int8>,
        price_mem -> Nullable<Float8>,
        price_step -> Nullable<Float8>,
        max_tx_ex_mem -> Nullable<Numeric>,
        max_tx_ex_steps -> Nullable<Numeric>,
        max_block_ex_mem -> Nullable<Numeric>,
        max_block_ex_steps -> Nullable<Numeric>,
        max_val_size -> Nullable<Numeric>,
        collateral_percent -> Nullable<Int4>,
        max_collateral_inputs -> Nullable<Int4>,
        registered_tx_id -> Int8,
    }
}

table! {
    pool_hash (id) {
        id -> Int8,
        hash_raw -> Bytea,
        view -> Varchar,
    }
}

table! {
    pool_metadata_ref (id) {
        id -> Int8,
        pool_id -> Int8,
        url -> Varchar,
        hash -> Bytea,
        registered_tx_id -> Int8,
    }
}

table! {
    pool_offline_data (id) {
        id -> Int8,
        pool_id -> Int8,
        ticker_name -> Varchar,
        hash -> Bytea,
        json -> Jsonb,
        bytes -> Bytea,
        pmr_id -> Int8,
    }
}

table! {
    pool_offline_fetch_error (id) {
        id -> Int8,
        pool_id -> Int8,
        fetch_time -> Timestamp,
        pmr_id -> Int8,
        fetch_error -> Varchar,
        retry_count -> Int4,
    }
}

table! {
    pool_owner (id) {
        id -> Int8,
        addr_id -> Int8,
        pool_hash_id -> Int8,
        registered_tx_id -> Int8,
    }
}

table! {
    pool_relay (id) {
        id -> Int8,
        update_id -> Int8,
        ipv4 -> Nullable<Varchar>,
        ipv6 -> Nullable<Varchar>,
        dns_name -> Nullable<Varchar>,
        dns_srv_name -> Nullable<Varchar>,
        port -> Nullable<Int4>,
    }
}

table! {
    pool_retire (id) {
        id -> Int8,
        hash_id -> Int8,
        cert_index -> Int4,
        announced_tx_id -> Int8,
        retiring_epoch -> Int4,
    }
}

table! {
    pool_update (id) {
        id -> Int8,
        hash_id -> Int8,
        cert_index -> Int4,
        vrf_key_hash -> Bytea,
        pledge -> Numeric,
        reward_addr -> Bytea,
        active_epoch_no -> Int8,
        meta_id -> Nullable<Int8>,
        margin -> Float8,
        fixed_cost -> Numeric,
        registered_tx_id -> Int8,
    }
}

table! {
    pot_transfer (id) {
        id -> Int8,
        cert_index -> Int4,
        treasury -> Numeric,
        reserves -> Numeric,
        tx_id -> Int8,
    }
}

table! {
    redeemer (id) {
        id -> Int8,
        tx_id -> Int8,
        unit_mem -> Int8,
        unit_steps -> Int8,
        fee -> Numeric,
        purpose -> crate::schema::sql_types::Scriptpurposetype,
        index -> Int4,
        script_hash -> Nullable<Bytea>,
        datum_id -> Int8,
    }
}

table! {
    reserve (id) {
        id -> Int8,
        addr_id -> Int8,
        cert_index -> Int4,
        amount -> Numeric,
        tx_id -> Int8,
    }
}

table! {
    reserved_pool_ticker (id) {
        id -> Int8,
        name -> Varchar,
        pool_hash -> Bytea,
    }
}

table! {
    reward (id) {
        id -> Int8,
        addr_id -> Int8,
        #[sql_name = "type"]
        type_ -> crate::schema::sql_types::Rewardtype,
        amount -> Numeric,
        earned_epoch -> Int8,
        spendable_epoch -> Int8,
        pool_id -> Nullable<Int8>,
    }
}

table! {
    schema_version (id) {
        id -> Int8,
        stage_one -> Int8,
        stage_two -> Int8,
        stage_three -> Int8,
    }
}

table! {
    script (id) {
        id -> Int8,
        tx_id -> Int8,
        hash -> Bytea,
        #[sql_name = "type"]
        type_ -> crate::schema::sql_types::Scripttype,
        json -> Nullable<Jsonb>,
        bytes -> Nullable<Bytea>,
        serialised_size -> Nullable<Int4>,
    }
}

table! {
    slot_leader (id) {
        id -> Int8,
        hash -> Bytea,
        pool_hash_id -> Nullable<Int8>,
        description -> Varchar,
    }
}

table! {
    stake_address (id) {
        id -> Int8,
        hash_raw -> Bytea,
        view -> Varchar,
        script_hash -> Nullable<Bytea>,
        registered_tx_id -> Int8,
    }
}

table! {
    stake_deregistration (id) {
        id -> Int8,
        addr_id -> Int8,
        cert_index -> Int4,
        epoch_no -> Int4,
        tx_id -> Int8,
        redeemer_id -> Nullable<Int8>,
    }
}

table! {
    stake_registration (id) {
        id -> Int8,
        addr_id -> Int8,
        cert_index -> Int4,
        epoch_no -> Int4,
        tx_id -> Int8,
    }
}

table! {
    treasury (id) {
        id -> Int8,
        addr_id -> Int8,
        cert_index -> Int4,
        amount -> Numeric,
        tx_id -> Int8,
    }
}

table! {
    tx (id) {
        id -> Int8,
        hash -> Bytea,
        block_id -> Int8,
        block_index -> Int4,
        out_sum -> Numeric,
        fee -> Numeric,
        deposit -> Int8,
        size -> Int4,
        invalid_before -> Nullable<Numeric>,
        invalid_hereafter -> Nullable<Numeric>,
        valid_contract -> Bool,
        script_size -> Int4,
    }
}

table! {
    tx_in (id) {
        id -> Int8,
        tx_in_id -> Int8,
        tx_out_id -> Int8,
        tx_out_index -> Int2,
        redeemer_id -> Nullable<Int8>,
    }
}

table! {
    tx_metadata (id) {
        id -> Int8,
        key -> Numeric,
        json -> Nullable<Jsonb>,
        bytes -> Bytea,
        tx_id -> Int8,
    }
}

table! {
    tx_out (id) {
        id -> Int8,
        tx_id -> Int8,
        index -> Int2,
        address -> Varchar,
        address_raw -> Bytea,
        address_has_script -> Bool,
        payment_cred -> Nullable<Bytea>,
        stake_address_id -> Nullable<Int8>,
        value -> Numeric,
        data_hash -> Nullable<Bytea>,
    }
}

table! {
    withdrawal (id) {
        id -> Int8,
        addr_id -> Int8,
        amount -> Numeric,
        redeemer_id -> Nullable<Int8>,
        tx_id -> Int8,
    }
}

joinable!(ada_pots -> block (block_id));
joinable!(block -> slot_leader (slot_leader_id));
joinable!(cost_model -> block (block_id));
joinable!(datum -> tx (tx_id));
joinable!(delegation -> pool_hash (pool_hash_id));
joinable!(delegation -> redeemer (redeemer_id));
joinable!(delegation -> stake_address (addr_id));
joinable!(delegation -> tx (tx_id));
joinable!(epoch_param -> block (block_id));
joinable!(epoch_param -> cost_model (cost_model_id));
joinable!(epoch_stake -> pool_hash (pool_id));
joinable!(epoch_stake -> stake_address (addr_id));
joinable!(ma_tx_mint -> multi_asset (ident));
joinable!(ma_tx_mint -> tx (tx_id));
joinable!(ma_tx_out -> multi_asset (ident));
joinable!(ma_tx_out -> tx_out (tx_out_id));
joinable!(param_proposal -> cost_model (cost_model_id));
joinable!(param_proposal -> tx (registered_tx_id));
joinable!(pool_metadata_ref -> pool_hash (pool_id));
joinable!(pool_metadata_ref -> tx (registered_tx_id));
joinable!(pool_offline_data -> pool_hash (pool_id));
joinable!(pool_offline_data -> pool_metadata_ref (pmr_id));
joinable!(pool_offline_fetch_error -> pool_hash (pool_id));
joinable!(pool_offline_fetch_error -> pool_metadata_ref (pmr_id));
joinable!(pool_owner -> pool_hash (pool_hash_id));
joinable!(pool_owner -> stake_address (addr_id));
joinable!(pool_owner -> tx (registered_tx_id));
joinable!(pool_relay -> pool_update (update_id));
joinable!(pool_retire -> pool_hash (hash_id));
joinable!(pool_retire -> tx (announced_tx_id));
joinable!(pool_update -> pool_hash (hash_id));
joinable!(pool_update -> pool_metadata_ref (meta_id));
joinable!(pool_update -> tx (registered_tx_id));
joinable!(pot_transfer -> tx (tx_id));
joinable!(redeemer -> datum (datum_id));
joinable!(redeemer -> tx (tx_id));
joinable!(reserve -> stake_address (addr_id));
joinable!(reserve -> tx (tx_id));
joinable!(reward -> pool_hash (pool_id));
joinable!(reward -> stake_address (addr_id));
joinable!(script -> tx (tx_id));
joinable!(slot_leader -> pool_hash (pool_hash_id));
joinable!(stake_address -> tx (registered_tx_id));
joinable!(stake_deregistration -> redeemer (redeemer_id));
joinable!(stake_deregistration -> stake_address (addr_id));
joinable!(stake_deregistration -> tx (tx_id));
joinable!(stake_registration -> stake_address (addr_id));
joinable!(stake_registration -> tx (tx_id));
joinable!(treasury -> stake_address (addr_id));
joinable!(treasury -> tx (tx_id));
joinable!(tx -> block (block_id));
joinable!(tx_in -> redeemer (redeemer_id));
joinable!(tx_metadata -> tx (tx_id));
joinable!(tx_out -> stake_address (stake_address_id));
joinable!(tx_out -> tx (tx_id));
joinable!(withdrawal -> redeemer (redeemer_id));
joinable!(withdrawal -> stake_address (addr_id));
joinable!(withdrawal -> tx (tx_id));

allow_tables_to_appear_in_same_query!(
    unspent_utxos,
    ada_pots,
    admin_user,
    block,
    collateral_tx_in,
    cost_model,
    datum,
    delegation,
    delisted_pool,
    epoch,
    epoch_param,
    epoch_reward_total_received,
    epoch_stake,
    epoch_sync_time,
    ma_tx_mint,
    ma_tx_out,
    meta,
    multi_asset,
    param_proposal,
    pool_hash,
    pool_metadata_ref,
    pool_offline_data,
    pool_offline_fetch_error,
    pool_owner,
    pool_relay,
    pool_retire,
    pool_update,
    pot_transfer,
    redeemer,
    reserve,
    reserved_pool_ticker,
    reward,
    schema_version,
    script,
    slot_leader,
    stake_address,
    stake_deregistration,
    stake_registration,
    treasury,
    tx,
    tx_in,
    tx_metadata,
    tx_out,
    withdrawal,
);
