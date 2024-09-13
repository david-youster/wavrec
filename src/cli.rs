use clap::Parser;

use crate::audio::SampleFormat;

#[derive(Parser)]
pub struct Args {
    pub file_name: String,

    #[arg(short, long, default_value = "int16")]
    pub format: SampleFormat,

    #[arg(short, long, default_value_t = 44100)]
    pub sample_rate: u32,
}
