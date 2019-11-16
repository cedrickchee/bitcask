// This example demonstrates clap's building from YAML style of creating arguments which is far
// more clean, but takes a very small performance hit compared to the other two methods.
//
// How to test in terminal. Run these commands:
// $ cargo run -- --help
// $ cargo run -- myfile.txt

use std::env;
use std::fmt;
use std::process;

#[macro_use]
extern crate clap;
use clap::App;

#[macro_use]
extern crate dotenv_codegen;

enum MyErr {
    Reason1(String),
    Reason2(String, u32),
}

impl fmt::Display for MyErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MyErr::Reason1(s) => write!(f, "`{}` is the error", s),
            MyErr::Reason2(s, num) => write!(f, "`{}` and `{}` are error", s, num),
        }
    }
}

fn get_env_vars() -> Result<(), MyErr> {
    let key = "HOME";
    match env::var_os(key) {
        Some(val) => {
            println!("{}: {:?}", key, val);
            Ok(())
        }
        None => Err(MyErr::Reason2(
            format!("{} is not defined in the environment.", key),
            1,
        )),
    }
}

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("cli.yaml");
    let _matches = App::from_yaml(yaml).get_matches();

    println!("PORT from your `.env` file is {}", dotenv!("PORT"));

    let path: &'static str = env!("PATH");
    println!("the $PATH variable at the time of compiling was: {}", path);

    match get_env_vars() {
        Ok(_) => println!("get environment variables successed."),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
