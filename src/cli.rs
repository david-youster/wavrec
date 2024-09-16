use clap::Parser;

use crate::audio::SampleFormat;

#[derive(Parser)]
pub struct Args {
    file_name: String,

    #[arg(short, long, default_value = "int16")]
    pub format: SampleFormat,

    #[arg(short, long, default_value_t = 44100)]
    pub sample_rate: u32,
}

impl Args {
    pub fn file_name(&self) -> String {
        if !self.file_name.ends_with(".wav") {
            return format!("{}.wav", &self.file_name[..]);
        };
        self.file_name.clone()
    }
}
