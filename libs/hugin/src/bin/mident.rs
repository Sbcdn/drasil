/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use hugin::encryption::mident;

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

    println!("{}", ident);
}
