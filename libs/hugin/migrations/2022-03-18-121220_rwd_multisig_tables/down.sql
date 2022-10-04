--#################################################################################
--# See LICENSE.md for full license information.                                  #
--# Software: Drasil Blockchain Application Framework                             #
--# License: Drasil Source Available License v1.0                                 #
--# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
--#################################################################################
ALTER TABLE public.multisig_keyloc DROP CONSTRAINT unique_multisig_keyloc;

DROP TABLE multisig_keyloc;

ALTER TABLE public.contracts DROP CONSTRAINT unique_address;
ALTER TABLE public.contracts DROP CONSTRAINT unique_contract;
