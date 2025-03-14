use clap::Parser;
use std::process::exit;

const UNIMPLEMENTED: &str = "unimplemented";

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), version, about, long_about = None)]
struct Args {
    get: Option<String>,
    rm: Option<String>,
    set: Option<Vec<String>>,
}

fn main() {
    let args = Args::parse();

    if args.get.is_some() {
        get_handler(&args.get.unwrap());
    } else if args.set.is_some() {
        set_handler(&args.set.unwrap());
    } else if args.rm.is_some() {
        rm_handler(&args.rm.unwrap());
    } else {
        exit(1);
    }
}

fn get_handler(value: &String) -> &str {
    eprintln!("{}", UNIMPLEMENTED);
    exit(1);
}

fn set_handler(value: &Vec<String>) -> &str {
    eprintln!("{}", UNIMPLEMENTED);
    exit(1);
}

fn rm_handler(value: &String) -> &str {
    eprintln!("{}", UNIMPLEMENTED);
    exit(1);
}
