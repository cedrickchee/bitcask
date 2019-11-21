use std::env;
use std::net::SocketAddr;
use std::process::exit;

use log::{debug, error, info, LevelFilter};
use structopt::clap::arg_enum;
use structopt::StructOpt;

use kvs::Result;

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
    let server = run(opts);
    match server {
        Err(e) => {
            error!("{}", e);
            exit(1)
        }
        Ok(()) => info!("Server running."),
    }
}

fn run(opt: Options) -> Result<()> {
    info!("kvs-server {}", env!("CARGO_PKG_VERSION"));
    info!("Storage engine: {}", opt.engine);
    info!("Listening on {}", opt.addr);

    match opt.engine {
        Engine::Kvs => debug!("storage engine: kvs"),
        Engine::Sled => debug!("storage engine: sled"),
    }

    Ok(())
}
