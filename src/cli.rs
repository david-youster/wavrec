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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_without_extension_is_modified() {
        let args = Args {
            file_name: String::from("somefile"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
        };

        assert_eq!(args.file_name(), "somefile.wav");
    }

    #[test]
    fn file_name_with_extension_is_unchanged() {
        let args = Args {
            file_name: String::from("somefile.wav"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
        };

        assert_eq!(args.file_name(), "somefile.wav");
    }

    #[test]
    fn including_extension_is_optional() {
        let args_1 = Args {
            file_name: String::from("somefile"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
        };

        let args_2 = Args {
            file_name: String::from("somefile.wav"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
        };

        assert_eq!(args_1.file_name(), args_2.file_name());
    }
}
