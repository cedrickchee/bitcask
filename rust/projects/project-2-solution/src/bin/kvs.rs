use kvs::{KvStore, Result};
use std::env::current_dir;
use structopt::StructOpt;

mod cli;
use cli::{Options, SubCommand};

fn main() -> Result<()> {
    let opts = Options::from_args();

    match opts.cmd {
        SubCommand::Get { key } => {
            let mut store = KvStore::open(current_dir()?)?;

            let output = match store.get(key)? {
                Some(value) => value,
                None => "Key not found".to_string(),
            };

            println!("{}", output);
        }
        SubCommand::Set { key, value } => {
            let mut store = KvStore::open(current_dir()?)?;
            store.set(key, value)?;
        }
        SubCommand::Rm { key } => {
            let mut store = KvStore::open(current_dir()?)?;
            store.remove(key)?;
        }
    }
    Ok(())
}
