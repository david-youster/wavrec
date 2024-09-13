use clap::Parser;

use crate::audio::SampleFormat;

#[derive(Parser)]
pub struct Args {
    pub file_name: String,

    #[arg(short, long, default_value = "int16")]
    pub format: SampleFormat,
}
