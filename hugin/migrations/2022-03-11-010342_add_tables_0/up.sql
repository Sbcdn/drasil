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

CREATE TABLE contracts (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    contract_id bigint NOT NULL,
    contract_type VARCHAR(20) NOT NULL,
    description VARCHAR(100),
    version REAL NOT NULL,
    plutus TEXT NOT NULL,
    address VARCHAR(80) NOT NULL,
    policy_id VARCHAR(120),
    depricated BOOLEAN NOT NULL DEFAULT false,
    drasil_lqdty BIGINT,
    customer_lqdty BIGINT,
    external_lqdty BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE drasil_user (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    api_pubkey VARCHAR(250), 
    uname VARCHAR(24) NOT NULL,
    email VARCHAR(150) NOT NULL,
    pwd TEXT NOT NULL,
    role VARCHAR(20) NOT NULL,
    permissions TEXT[] NOT NULL,
    company_name VARCHAR(32),
    address VARCHAR(128),
    post_code VARCHAR(12),
    city VARCHAR(100),
    addional_addr VARCHAR (128),
    country VARCHAR(30),
    contact_p_fname VARCHAR(50),
    contact_p_sname VARCHAR(50),
    contact_p_tname VARCHAR(50),
    identification TEXT[] NOT NULL,
    email_verified BOOLEAN NOT NULL,
    cardano_wallet TEXT,
    cwallet_verified BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


CREATE TABLE email_verification_token (
    id BYTEA PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);    

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON contracts
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON drasil_user
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON email_verification_token
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

ALTER TABLE contracts ADD CONSTRAINT unique_contract UNIQUE(user_id, contract_id, version);
ALTER TABLE contracts ADD CONSTRAINT unique_address UNIQUE(address);
ALTER TABLE drasil_user ADD CONSTRAINT unique_email UNIQUE(email);
ALTER TABLE drasil_user ADD CONSTRAINT unique_user_id UNIQUE(user_id);
ALTER TABLE drasil_user ADD CONSTRAINT unique_api_key UNIQUE(api_pubkey);
