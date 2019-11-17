use structopt::StructOpt;

mod cli;
use cli::{Options, SubCommand};

fn main() {
    let opts = Options::from_args();

    match opts.cmd {
        SubCommand::Get { key } => {
            println!("get - key: {}", key);
            panic!("unimplemented")
        }
        SubCommand::Set { key, value } => {
            println!("set - key: {}, value: {}", key, value);
            panic!("unimplemented")
        }
        SubCommand::Rm { key } => {
            println!("remove - key: {}", key);
            panic!("unimplemented")
        }
    }
}
