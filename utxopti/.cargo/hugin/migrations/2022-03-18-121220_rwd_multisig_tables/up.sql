--#################################################################################
--# See LICENSE.md for full license information.                                  #
--# Software: Drasil Blockchain Application Framework                             #
--# License: Drasil Source Available License v1.0                                 #
--# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
--#################################################################################


CREATE TABLE multisig_keyloc (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    contract_id bigint NOT NULL,
    version REAL NOT NULL,
    fee_wallet_addr VARCHAR(120),
    fee bigint, 
    pvks TEXT[] NOT NULL,
    depricated BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


ALTER TABLE multisig_keyloc ADD CONSTRAINT unique_multisig_keyloc UNIQUE(user_id,contract_id,version);
