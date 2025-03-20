use clap::{Parser, Subcommand};
use kvs::{KvStore, Result};
use std::process::exit;

const UNIMPLEMENTED: &str = "unimplemented";

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Get { key: String },
    Set { key: String, value: String },
    Rm { key: String },
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut kvs = KvStore::new();

    match &args.command {
        Commands::Get { key } => {
            get_handler(key, &mut kvs);
        }
        Commands::Set { key, value } => {
            set_handler(key.to_string(), value.to_string(), &mut kvs);
        }
        Commands::Rm { key } => {
            rm_handler(key, &mut kvs);
        }
    }

    panic!()
}

fn get_handler<'a>(value: &'a str, kvs: &'a mut KvStore) {
    match kvs.get(value.to_string()) {
        Ok(result) => match result {
            Some(inner_result) => {
                println!("{}", inner_result);
                exit(0)
            }
            None => {
                println!("Key not found");
                exit(0)
            }
        },
        Err(error) => {
            eprint!("{}", error);
            exit(1)
        }
    }
}

fn set_handler(key: String, value: String, kvs: &mut KvStore) {
    match kvs.set(key, value) {
        Ok(_) => exit(0),
        Err(error) => {
            eprint!("{}", error);
            exit(1)
        }
    }
}

fn rm_handler<'a>(value: &'a str, kvs: &'a mut KvStore) {
    eprintln!("{}", UNIMPLEMENTED);
    exit(1);
}
