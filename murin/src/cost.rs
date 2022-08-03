use cardano_serialization_lib::{
    plutus::RedeemerTagKind,
    utils::{from_bignum, TransactionUnspentOutputs},
    Transaction,
};
use uplc;

use crate::ExUnit;

pub fn calculate(tx: Transaction, input_utxos: TransactionUnspentOutputs) -> ExUnit {
    let mut ex_unit = ExUnit {
        steps: 0.,
        memory: 0.,
    };

    let witness_set = tx.witness_set();

    if let Some((scripts, redeemers)) = witness_set.plutus_scripts().zip(witness_set.redeemers()) {
        for i in 0..scripts.len() {
            let script = scripts.get(i);
            let redeemer = redeemers.get(i);

            let program: uplc::ast::Program<uplc::ast::DeBruijn> =
                uplc::ast::Program::from_flat(&script.bytes()).unwrap();

            let program: uplc::ast::Program<uplc::ast::NamedDeBruijn> = program.try_into().unwrap();

            let program = match redeemer.tag().kind() {
                RedeemerTagKind::Mint => todo!(),
                RedeemerTagKind::Spend => {
                    let input = tx
                        .body()
                        .inputs()
                        .get(from_bignum(&redeemer.index()) as usize);

                    let output = input_utxos
                        .get(from_bignum(&redeemer.index()) as usize)
                        .output();

                    match output.plutus_data() {
                        Some(data) => program.apply(data),
                        None => todo!(),
                    }
                }
                RedeemerTagKind::Cert => todo!(),
                RedeemerTagKind::Reward => todo!(),
            };

            let program = program.apply(redeemer.data());

            // TODO: apply script context

            let (_result, ex_budget, _logs) = program.eval();

            ex_unit.steps += ex_budget.cpu as f64;
            ex_unit.memory += ex_budget.mem as f64;
        }
    }

    ex_unit
}
