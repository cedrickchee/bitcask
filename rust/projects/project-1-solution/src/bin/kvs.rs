#[macro_use]
extern crate clap;
use clap::App;

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml)
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .get_matches();

    match matches.subcommand_name() {
        Some("get") => panic!("unimplemented"),
        Some("set") => panic!("unimplemented"),
        Some("rm") => panic!("unimplemented"),
        None => panic!("No subcommand was used"),
        _ => panic!("Some other subcommand was used"),
    }
}
