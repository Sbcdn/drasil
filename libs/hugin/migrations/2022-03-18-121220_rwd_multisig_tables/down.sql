ALTER TABLE public.multisig_keyloc DROP CONSTRAINT unique_multisig_keyloc;

DROP TABLE multisig_keyloc;

ALTER TABLE public.contracts DROP CONSTRAINT unique_address;
ALTER TABLE public.contracts DROP CONSTRAINT unique_contract;
