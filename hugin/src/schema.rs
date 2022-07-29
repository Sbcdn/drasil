/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
table! {
    contracts (id) {
        id -> Int8,
        user_id -> Int8,
        contract_id -> Int8,
        contract_type -> Varchar,
        description -> Nullable<Varchar>,
        version -> Float4,
        plutus -> Text,
        address -> Varchar,
        policy_id -> Nullable<Varchar>,
        depricated -> Bool,
        drasil_lqdty    -> Nullable<Int8>,
        customer_lqdty  -> Nullable<Int8>,
        external_lqdty  -> Nullable<Int8>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    drasil_user (id) {
        id -> Int8,
        user_id -> Int8,
        api_pubkey -> Nullable<Varchar>,
        uname -> Varchar,
        email -> Varchar,
        pwd -> Text,
        role -> Varchar,
        permissions -> Array<Text>,
        company_name -> Nullable<Varchar>,
        address -> Nullable<Varchar>,
        post_code -> Nullable<Varchar>,
        city -> Nullable<Varchar>,
        addional_addr -> Nullable<Varchar>,
        country -> Nullable<Varchar>,
        contact_p_fname -> Nullable<Varchar>,
        contact_p_sname -> Nullable<Varchar>,
        contact_p_tname -> Nullable<Varchar>,
        identification -> Array<Text>,
        email_verified -> Bool,
        cardano_wallet -> Nullable<Text>,
        cwallet_verified -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    multisig_keyloc (id) {
        id -> Int8,
        user_id -> Int8,
        contract_id -> Int8,
        version -> Float4,
        fee_wallet_addr -> Nullable<Varchar>,
        fee -> Nullable<Int8>,
        pvks -> Array<Text>,
        depricated -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    multisigs (id) {
        id -> Int8,
        user_id -> Int8,
        contract_id -> Int8,
        description -> Nullable<Varchar>,
        version -> Float4,
        multisig -> Text,
        depricated -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    email_verification_token (id) {
        id -> Bytea,
        email -> Text,
        expires_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

allow_tables_to_appear_in_same_query!(
    contracts,
    drasil_user,
    multisig_keyloc,
    multisigs,
    email_verification_token,
);
