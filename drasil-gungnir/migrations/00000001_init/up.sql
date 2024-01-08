
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TYPE public.calculationmode AS ENUM (
    'custom',
    'modifactorandequation',
    'simpleequation',
    'fixedendepoch',
    'relationaltoadastake',
    'airdrop'
);

CREATE DOMAIN public.amount AS numeric(20,0)
	CONSTRAINT amount_check CHECK (((VALUE >= (0)::numeric) AND (VALUE <= '18446744073709551615'::numeric)));


CREATE TABLE rewards (
    id BIGSERIAL PRIMARY KEY,
    stake_addr VARCHAR(100) NOT NULL,
    payment_addr VARCHAR(200) NOT NULL,
    fingerprint VARCHAR(100) NOT NULL,
    contract_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    tot_earned amount NOT NULL,
    tot_claimed amount NOT NULL,
    oneshot BOOLEAN NOT NULL,
    last_calc_epoch BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE claimed (
    id BIGSERIAL PRIMARY KEY,
    stake_addr VARCHAR(100) NOT NULL,
    payment_addr VARCHAR(140) NOT NULL,
    fingerprint VARCHAR(100) NOT NULL,
    amount amount NOT NULL,
    contract_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    txhash VARCHAR(120) NOT NULL,
    invalid BOOLEAN,
    invalid_descr TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


CREATE TABLE token_whitelist (
    id BIGSERIAL PRIMARY KEY,
    fingerprint VARCHAR(100),
    policy_id VARCHAR(100) NOT NULL,
    tokenname VARCHAR(140),
    contract_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    vesting_period TIMESTAMPTZ NOT NULL DEFAULT NOW(), -- All new entries are valid on creation, vesting has to be set explicitly
    pools  TEXT[] NOT NULL,
    mode calculationmode NOT NULL,
    equation TEXT NOT NULL,
    start_epoch BIGINT NOT NULL,
    end_epoch BIGINT,
    modificator_equ TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TYPE public.calculationmode ADD VALUE  IF NOT EXISTS 'custom' BEFORE 'fixedamountperepoch';

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


CREATE TRIGGER set_timestamp
BEFORE UPDATE ON rewards
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON claimed
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON token_whitelist
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

ALTER TABLE rewards ADD CONSTRAINT unique_stake_per_contract UNIQUE(stake_addr, fingerprint, contract_id,user_id);
ALTER TABLE claimed ADD CONSTRAINT unique_txhash UNIQUE(txhash,fingerprint);
ALTER TABLE token_whitelist ADD CONSTRAINT unique_token_per_contract_and_user UNIQUE(user_id,contract_id,policy_id,fingerprint);
