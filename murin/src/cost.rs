use cardano_serialization_lib::Transaction;
use uplc;

use crate::ExUnit;

pub fn calculate(tx: Transaction) -> ExUnit {
    let mut ex_unit = ExUnit {
        steps: 0.,
        memory: 0.,
    };

    if let Some(scripts) = tx.witness_set().plutus_scripts() {
        for i in 0..scripts.len() {
            let script = scripts.get(i);

            let program: uplc::ast::Program<uplc::ast::DeBruijn> =
                uplc::ast::Program::from_flat(&script.bytes()).unwrap();

            let program: uplc::ast::Program<uplc::ast::NamedDeBruijn> = program.try_into().unwrap();

            let (_result, ex_budget, _logs) = program.eval();

            ex_unit.steps += ex_budget.cpu as f64;
            ex_unit.memory += ex_budget.mem as f64;
        }
    }

    ex_unit
}
