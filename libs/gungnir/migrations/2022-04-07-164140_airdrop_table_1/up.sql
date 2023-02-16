--#################################################################################
--# See LICENSE.md for full license information.                                  #
--# Software: Drasil Blockchain Application Framework                             #
--# License: Drasil Source Available License v1.0                                 #
--# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
--#################################################################################

CREATE TABLE airdrop_whitelist (
    id BIGSERIAL PRIMARY KEY,
    contract_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    reward_created BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


CREATE TABLE airdrop_parameter (
    id BIGSERIAL PRIMARY KEY,
    contract_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    airdrop_type_type VARCHAR(3) NOT NULL,
    distribution_type VARCHAR(100) NOT NULL,
    selection_type TEXT NOT NULL,
    args_1 TEXT[] NOT NULL,
    args_2 TEXT[] NOT NULL,
    args_3 TEXT[] NOT NULL,
    whitelist_ids BIGINT[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON airdrop_whitelist
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON airdrop_parameter
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TYPE public.whitelisttype AS ENUM (
    'RandomContained',
    'SpecificAsset',
    'RandomPreallocated'
);

CREATE TABLE wladdresses (
    id BIGSERIAL PRIMARY KEY,
    payment_address VARCHAR NOT NULL UNIQUE,
    stake_address VARCHAR
);

CREATE TABLE wlalloc (
    wl BIGINT NOT NULL,
    addr BIGINT NOT NULL,
    specific_asset Jsonb,
    PRIMARY KEY(wl,addr,specific_asset)
);
ALTER TABLE wlalloc ADD CONSTRAINT unique_token_per_whitelist UNIQUE(wl, specific_asset);

CREATE TABLE whitelist (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    max_addr_repeat INTEGER NOT NULL,
    wl_type whitelisttype NOT NULL,
    description VARCHAR NOT NULL,
    notes VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON whitelist
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TABLE mint_projects (
    id BIGSERIAL PRIMARY KEY,
    project_name VARCHAR NOT NULL,
    user_id BIGINT NOT NULL,
    mint_contract_id BIGINT NOT NULL,
    whitelists BIGINT[],
    mint_start_date TIMESTAMPTZ NOT NULL,
    mint_end_date TIMESTAMPTZ,
    storage_type Varchar NOT NULL,
    storage_url Varchar,
    storage_access_token Varchar,
    collection_name VARCHAR NOT NULL,
    author VARCHAR(64) NOT NULL,
    meta_description VARCHAR(64) NOT NULL,
    meta_common_nft_name VARCHAR(64),
    max_mint_p_addr INTEGER,
    nft_table_name VARCHAR(64) NOT NULL UNIQUE,
    active BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id,mint_contract_id),
    UNIQUE (user_id,collection_name)
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON mint_projects
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TABLE nft_table (
    project_id BIGINT NOT NULL,
    asset_name_b BYTEA NOT NULL,
    asset_name VARCHAR NOT NULL,
    fingerprint VARCHAR NOT NULL,
    nft_id VARCHAR NOT NULL,
    file_name VARCHAR NOT NULL,
    ipfs_hash VARCHAR,
    metadata TEXT NOT NULL,
    claim_addr VARCHAR,
    minted BOOLEAN NOT NULL,
    tx_hash VARCHAR,
    confirmed BOOLEAN,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY(project_id,asset_name_b)
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON nft_table
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE INDEX nft_project ON nft_table (project_id);

-- https://www.postgresql.org/docs/current/tsm-system-rows.html
CREATE EXTENSION tsm_system_rows;

CREATE TABLE discount (
    id BIGSERIAL PRIMARY KEY,
    contract_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    policy_id VARCHAR NOT NULL,
    fingerprint VARCHAR,
    metadata_path VARCHAR[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id,contract_id,policy_id)
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON discount
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TABLE mint_rewards (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL,
    pay_addr VARCHAR NOT NULL,
    nft_ids Bytea[] NOT NULL,
    v_nfts_b Bytea[] NOT NULL,
    processed BOOLEAN NOT NULL,
    minted BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id,nft_ids)
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON mint_rewards
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();