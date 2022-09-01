use structopt::StructOpt;
use utxopti::{optimize, Result};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "UTxO Optimizer",
    about = "Splitts large UTxOs into many small UTxOs"
)]
struct Opt {}

#[tokio::main]
pub async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    // Do Optimization for
    let addr = &"addr_test1wr84fwh5mt0usmwewfmzz5l0qyxrxa897eswwmrtcxz3mls9mwcxy".to_string();
    let uid = 0;
    let cid = 1;

    optimize(addr, uid, cid).await?;
    Ok(())
}
