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

CREATE TABLE wladdresses (
    id BIGSERIAL PRIMARY KEY,
    payment_address BIGINT NOT NULL,
    UNIQUE(payment_address)
);

CREATE TABLE wlalloc (
    wl BIGINT NOT NULL,
    addr BIGINT NOT NULL,
    PRIMARY KEY(wl,addr)
);

CREATE TABLE whitelist (
    id BIGSERIAL PRIMARY KEY,
    max_addr_repeat INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON whitelist
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TABLE mint_projects (
    id BIGSERIAL PRIMARY KEY,
    customer_name VARCHAR NOT NULL,
    project_name VARCHAR NOT NULL,
    user_id BIGINT NOT NULL,
    contract_id BIGINT NOT NULL,
    whitelist_id BIGINT,
    mint_start_date TIMESTAMPTZ NOT NULL,
    mint_end_date TIMESTAMPTZ,
    storage_folder Varchar NOT NULL,
    max_trait_count INTEGER NOT NULL,
    collection_name VARCHAR NOT NULL,
    author VARCHAR(64) NOT NULL,
    meta_description VARCHAR(64) NOT NULL,
    max_mint_p_addr INTEGER,
    reward_minter BOOLEAN,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id,contract_id)
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON mint_projects
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TABLE nft_table (
    project_id BIGINT NOT NULL,
    asset_name_b BYTEA NOT NULL,
    asset_name VARCHAR NOT NULL,
    picture_id VARCHAR NOT NULL,
    file_name VARCHAR NOT NULL,
    ipfs_hash VARCHAR,
    trait_category TEXT[] NOT NULL,
    traits TEXT[][] NOT NULL,
    metadata TEXT NOT NULL,
    payment_addr VARCHAR,
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