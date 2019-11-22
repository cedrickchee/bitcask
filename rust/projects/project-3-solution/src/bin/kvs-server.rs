#[macro_use]
extern crate log;

use std::env;
use std::net::SocketAddr;
use std::process::exit;

use log::LevelFilter;
use structopt::clap::arg_enum;
use structopt::StructOpt;

use kvs::{KvStore, KvsEngine, KvsServer, Result, SledKvsEngine};

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
        Engine::Kvs => run_with_engine(KvStore::open(env::current_dir()?)?, opt.addr)?,
        Engine::Sled => run_with_engine(
            SledKvsEngine::new(sled::Db::open(env::current_dir()?)?),
            opt.addr,
        )?,
    }

    Ok(())
}

fn run_with_engine<E: KvsEngine>(engine: E, addr: SocketAddr) -> Result<()> {
    // The trait `KvsEngine` is implemented for `KvStore`. So, the trait
    // bound `KvStore: KvsEngine` is satisfied.
    let server = KvsServer::new(engine);
    server.run(addr)
}
