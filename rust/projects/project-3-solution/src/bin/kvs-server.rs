#[macro_use]
extern crate log;

use std::env;
use std::net::SocketAddr;
use std::process::exit;

use log::LevelFilter;
use structopt::clap::arg_enum;
use structopt::StructOpt;

use kvs::{KvsServer, Result};

// A struct to hold command line arguments parsed.
#[derive(StructOpt, Debug)]
#[structopt(name = "kvs-server")]
pub struct Options {
    /// Sets the listening address
    #[structopt(long, value_name = "IP:PORT", default_value = "127.0.0.1:4000")]
    addr: SocketAddr,
    /// Sets the storage engine
    #[structopt(
        long,
        value_name = "ENGINE-NAME",
        default_value = "kvs",
        case_insensitive = true,
        possible_values = &Engine::variants()
    )]
    engine: Engine,
}

arg_enum! {
    #[derive(Debug)]
    enum Engine {
        Kvs,
        Sled,
    }
}

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let opts = Options::from_args();
    if let Err(e) = run(opts) {
        error!("{}", e);
        exit(1)
    }
}

fn run(opt: Options) -> Result<()> {
    info!("kvs-server {}", env!("CARGO_PKG_VERSION"));
    info!("Storage engine: {}", opt.engine);
    info!("Listening on {}", opt.addr);

    match opt.engine {
        Engine::Kvs => {
            let server = KvsServer::new();
            server.run(opt.addr)?;
        }
        Engine::Sled => debug!("sled engine"),
    }

    Ok(())
}
