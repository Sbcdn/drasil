-- Your SQL goes here

CREATE OR REPLACE VIEW public.unspent_utxos
AS SELECT tx_out.id,
    tx_out.tx_id,
    generating_tx.hash,
    tx_out.index,
    tx_out.address,
    tx_out.value,
    tx_out.data_hash,
    sa.view AS stake_address
   FROM tx_out
     JOIN tx generating_tx ON generating_tx.id = tx_out.tx_id
     left JOIN stake_address sa ON sa.id = tx_out.stake_address_id
     left JOIN tx_in consuming_input ON consuming_input.tx_out_id = generating_tx.id AND consuming_input.tx_out_index::smallint = tx_out.index::smallint
  WHERE consuming_input.tx_in_id IS NULL;

GRANT SELECT ON ALL TABLES IN SCHEMA public TO artifct_testnet;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO tp;