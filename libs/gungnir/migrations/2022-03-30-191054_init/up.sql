--#################################################################################
--# See LICENSE.md for full license information.                                  #
--# Software: Drasil Blockchain Application Framework                             #
--# License: Drasil Source Available License v1.0                                 #
--# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
--#################################################################################

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