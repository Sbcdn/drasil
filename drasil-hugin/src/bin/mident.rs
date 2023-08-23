use drasil_hugin::encryption::mident;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "fingerprint maker",
    about = "Creates Fingerprint from PolicyId and Tokenname"
)]
struct Opt {
    #[structopt(short, long)]
    u: i64,

    #[structopt(short, long)]
    c: i64,

    #[structopt(short, long)]
    v: f32,

    #[structopt(short, long)]
    a: String,
}

fn main() {
    let opt = Opt::from_args();

    let ident = mident(&opt.u, &opt.c, &opt.v, &opt.a);

    println!("{ident}");
}
