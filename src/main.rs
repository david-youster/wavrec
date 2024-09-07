use clap::Parser;
use std::process;
use wavrec::{run, Args};

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("Application failed with error: {:#?}", err);
        process::exit(1);
    }
}
