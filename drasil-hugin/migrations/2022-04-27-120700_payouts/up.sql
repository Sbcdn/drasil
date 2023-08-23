CREATE TABLE ca_payment (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    contract_id bigint NOT NULL,
    value VARCHAR NOT NULL,
    tx_hash VARCHAR(64),
    user_appr VARCHAR,
    drasil_appr VARCHAR,
    status_bl VARCHAR,
    status_pa VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


CREATE TABLE ca_payment_hash (
    id BIGSERIAL PRIMARY KEY,
    payment_id BIGINT NOT NULL,
    payment_hash VARCHAR NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


CREATE TRIGGER set_timestamp
BEFORE UPDATE ON ca_payment
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON ca_payment_hash
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();