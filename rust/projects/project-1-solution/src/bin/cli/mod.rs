use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "kvs")]
/// A struct to hold command line arguments parsed.
pub struct Options {
    #[structopt(subcommand)]
    pub cmd: SubCommand,
}

#[derive(StructOpt, Debug)]
pub enum SubCommand {
    /// Get the string value of a given string key
    Get {
        #[structopt(name = "KEY", required = true)]
        /// A string key
        key: String,
    },
    /// Set the value of a string key to a string
    Set {
        #[structopt(name = "KEY", required = true)]
        /// A string key
        key: String,
        #[structopt(name = "VALUE", required = true)]
        /// The string value of the key
        value: String,
    },
    /// Remove a given key
    Rm {
        #[structopt(name = "KEY", required = true)]
        /// A string key
        key: String,
    },
}
