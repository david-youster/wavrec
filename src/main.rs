//! WAV Recorder is a CLI application that can be used to capture the audio playback from a Windows
//! device and record it to a WAV file. The output audio format is configurable based on various
//! options made available through WASAPI.
//!
//! Run the application with the `-h` flag for detailed information on the available options.
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
