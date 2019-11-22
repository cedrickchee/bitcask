use std::process::exit;
use structopt::StructOpt;

use kvs::{KvsClient, Result};

mod cli;
use cli::{Options, SubCommand};

fn main() {
    let opts = Options::from_args();
    if let Err(e) = run(opts) {
        eprintln!("{}", e);
        exit(1);
    }
}

fn run(opts: Options) -> Result<()> {
    match opts.cmd {
        SubCommand::Get { key, addr } => {
            let mut client = KvsClient::connect(addr)?;

            let output = match client.get(key)? {
                Some(value) => value,
                None => "Key not found".to_string(),
            };

            println!("{}", output);
        }
        SubCommand::Set { key, value, addr } => {
            let mut client = KvsClient::connect(addr)?;
            client.set(key, value)?;
        }
        SubCommand::Rm { key, addr } => {
            let mut client = KvsClient::connect(addr)?;
            client.remove(key)?;
        }
    }
    Ok(())
}
