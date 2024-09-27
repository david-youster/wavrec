use clap::Parser;
use env_logger::Builder;
use log::{error, info};
use std::process;
use wavrec::{cli::Args, run};

fn main() {
    let args = Args::parse();

    Builder::new().filter_level(args.log_level()).init();
    info!("Starting loopback recorder application");

    if let Err(err) = run(args) {
        error!("Application failed with error: {err}");
        process::exit(1);
    }
}
