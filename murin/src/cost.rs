use cardano_serialization_lib::{
    plutus::RedeemerTagKind,
    utils::{from_bignum, TransactionUnspentOutputs},
    Transaction,
};
use pallas_primitives::{babbage::PlutusData, Fragment};
use uplc::ast::{Constant, DeBruijn, FakeNamedDeBruijn, NamedDeBruijn, Program, Term};

use crate::ExUnit;

pub fn calculate(
    tx: Transaction,
    input_utxos: TransactionUnspentOutputs,
) -> Result<ExUnit, Box<dyn std::error::Error>> {
    let mut ex_unit = ExUnit {
        steps: 0.,
        memory: 0.,
    };

    let witness_set = tx.witness_set();

    if let Some((scripts, redeemers)) = witness_set.plutus_scripts().zip(witness_set.redeemers()) {
        for i in 0..scripts.len() {
            let script = scripts.get(i);
            let redeemer = redeemers.get(i);

            let program = Program::<FakeNamedDeBruijn>::from_flat(&script.bytes())?;

            let program: Program<NamedDeBruijn> = program.into();

            let program = match redeemer.tag().kind() {
                RedeemerTagKind::Mint => todo!(),
                RedeemerTagKind::Spend => {
                    let input = tx
                        .body()
                        .inputs()
                        .get(from_bignum(&redeemer.index()) as usize);

                    let output = input_utxos.get(input.index() as usize).output();

                    match output.plutus_data() {
                        Some(data) => {
                            let datum = Program {
                                version: (0, 0, 0),
                                term: Term::Constant(Constant::Data(PlutusData::decode_fragment(
                                    &data.to_bytes(),
                                )?)),
                            };

                            program.apply(&datum)
                        }
                        None => todo!(),
                    }
                }
                RedeemerTagKind::Cert => todo!(),
                RedeemerTagKind::Reward => todo!(),
            };

            let redeemer = Program {
                version: (0, 0, 0),
                term: Term::Constant(Constant::Data(PlutusData::decode_fragment(
                    &redeemer.data().to_bytes(),
                )?)),
            };

            let program = program.apply(&redeemer);

            // TODO: apply script context

            let (_result, ex_budget, _logs) = program.eval();

            ex_unit.steps += ex_budget.cpu as f64;
            ex_unit.memory += ex_budget.mem as f64;
        }
    }

    Ok(ex_unit)
}
