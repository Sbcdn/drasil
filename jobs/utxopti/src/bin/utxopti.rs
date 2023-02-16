use structopt::StructOpt;
use tokio::task::JoinSet;
use utxopti::{optimize, Result};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "UTxO Optimizer",
    about = "Splitts large UTxOs into many small UTxOs"
)]
struct Opt {}

#[tokio::main]
pub async fn main() -> Result<()> {
    pretty_env_logger::init();
    println!("Start UTxO optimization");
    let contracts = hugin::TBContracts::get_all_active_rwd_contracts()?;

    let mut threads = JoinSet::new();
    println!("Checking contracts...");
    'outer: for contract in contracts {
        let _lq = contract.get_contract_liquidity();
        let utxos =
            mimir::get_address_utxos(&mut mimir::establish_connection()?, &contract.address)?;

        let twl = gungnir::TokenWhitelist::get_whitelist()?;

        if twl.iter().fold(true, |mut acc, n| {
            acc = acc
                && utxos
                    .clone()
                    .find_utxo_containing_policy(&n.policy_id)
                    .unwrap()
                    .len()
                    > 250;
            acc
        }) {
            continue 'outer;
        }
        println!("Push thread for contract {:?}...", &contract.address);
        threads.spawn(tokio::spawn(async move {
            match optimize(
                contract.address.clone(),
                contract.user_id,
                contract.contract_id,
            )
            .await
            {
                Ok(_) => {
                    println!("Optimization of contract {} successful", contract.address)
                }
                Err(e) => {
                    println!(
                        "Optimization of contract {} failed with: {:?}",
                        contract.address,
                        e.to_string()
                    );
                }
            };
        }));
    }

    while let Some(res) = threads.join_next().await {
        res.unwrap().unwrap();
    }

    Ok(())
}
