use clap::Parser;
use kvs::{KvStore, Result};
use std::process::exit;

const UNIMPLEMENTED: &str = "unimplemented";

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = None)]
    get: Option<String>,
    #[arg(short, long, default_value = None)]
    rm: Option<String>,
    #[arg(short, long, default_value = None, num_args = 2, value_delimiter = ' ')]
    set: Option<Vec<String>>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut kvs = KvStore::new();

    if args.get.is_some() {
        get_handler(&args.get.unwrap(), &mut kvs);
    } else if args.set.is_some() {
        set_handler(&args.set.unwrap(), &mut kvs);
    } else if args.rm.is_some() {
        rm_handler(&args.rm.unwrap(), &mut kvs);
    } else {
        exit(1);
    }

    panic!()
}

fn get_handler<'a>(value: &'a str, kvs: &'a mut KvStore) -> &'a str {
    eprintln!("{}", UNIMPLEMENTED);
    exit(1);
}

fn set_handler(value: &[String], kvs: &mut KvStore) {
    match kvs.set(value[0].clone(), value[1].clone()) {
        Ok(_) => exit(0),
        Err(error) => {
            eprint!("{}", error);
            exit(1)
        }
    }
}

fn rm_handler<'a>(value: &'a str, kvs: &'a mut KvStore) -> &'a str {
    eprintln!("{}", UNIMPLEMENTED);
    exit(1);
}
