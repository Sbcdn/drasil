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
    // Do Optimization for
    let addr = &"addr_test1wzmtce0gj4jqm08n9tar4nq9n4t9z3sv5jzey66xh8zrvsg0l6anc".to_string();
    let uid = 0;
    let cid = 1;

    optimize(addr, uid, cid).await?;
    Ok(())
}
