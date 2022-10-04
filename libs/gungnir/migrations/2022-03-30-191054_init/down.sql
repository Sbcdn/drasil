--#################################################################################
--# See LICENSE.md for full license information.                                  #
--# Software: Drasil Blockchain Application Framework                             #
--# License: Drasil Source Available License v1.0                                 #
--# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
--#################################################################################


DROP TABLE rewards;
DROP TABLE claimed;
DROP TABLE token_whitelist;

DROP FUNCTION trigger_set_timestamp();
DROP TYPE public.amount;
DROP TYPE public.calculationmode;

