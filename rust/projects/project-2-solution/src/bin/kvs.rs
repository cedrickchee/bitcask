use kvs::Result;
use std::process::exit;
use structopt::StructOpt;

mod cli;
use cli::{Options, SubCommand};

fn main() -> Result<()> {
    let opts = Options::from_args();

    match opts.cmd {
        SubCommand::Get { .. } => {
            eprintln!("unimplemented");
            exit(1);
        }
        SubCommand::Set { .. } => {
            eprintln!("unimplemented");
            exit(1);
        }
        SubCommand::Rm { .. } => {
            eprintln!("unimplemented");
            exit(1);
        }
    }
}
