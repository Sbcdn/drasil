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
    let addr = &"addr1wy95d9ts6ut0fwjfcfyvkfkrs04x8g5xlvakjgs06u00z0sejufrj".to_string();
    let uid = 0;
    let cid = 1;

    optimize(addr, uid, cid).await?;
    Ok(())
}
