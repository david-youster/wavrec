use clap::{Parser, ValueEnum};
use log::LevelFilter;

use crate::audio::SampleFormat;

#[derive(ValueEnum, Clone, Copy)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Command line arguments, courtesty of [`clap`]. Implements [`clap::Parser`].
#[derive(Parser)]
pub struct Args {
    /// The file name to write to. Extension `.wav` will be appended if not specified.
    file_name: String,

    /// Sample format to write. Supports signed integer and float audio of various bit depths.
    /// This value will be requested from the audio device, and will determine the format of the
    /// output WAV file.
    #[arg(short, long, default_value = "int16")]
    pub format: SampleFormat,

    /// The sample rate. This will be requested from the audio device stream, and will determine
    /// the format of the output file.
    #[arg(short, long, default_value_t = 44100)]
    pub sample_rate: u32,

    /// Number of audio channels. This will be requested from the audio device stream, and will
    /// determine the format of the output WAV.
    #[arg(
        short,
        long,
        default_value_t = 2,
        help = "Number of channels to capture"
    )]
    pub channels: u8,

    /// The log level. `Off` to disable, `Trace` is the most  granular.
    /// Corresponds to [`log::LevelFilter`] values.
    #[arg(short, long, default_value = "info")]
    log_level: LogLevel,
}

impl Args {
    /// Get the file name to write to. If file name is missing extension, it will be appended here.
    pub fn file_name(&self) -> String {
        if !self.file_name.ends_with(".wav") {
            return format!("{}.wav", &self.file_name[..]);
        };
        self.file_name.clone()
    }

    /// Map the log level config property to a [`log::LevelFilter`] value.
    pub fn log_level(&self) -> LevelFilter {
        match self.log_level {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_file_name_without_extension_is_modified() {
        let args = Args {
            file_name: String::from("somefile"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
            channels: 2,
            log_level: LogLevel::Info,
        };

        assert_eq!(args.file_name(), "somefile.wav");
    }

    #[test]
    fn test_file_name_with_extension_is_unchanged() {
        let args = Args {
            file_name: String::from("somefile.wav"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
            channels: 2,
            log_level: LogLevel::Info,
        };

        assert_eq!(args.file_name(), "somefile.wav");
    }

    #[test]
    fn test_including_extension_is_optional() {
        let args_1 = Args {
            file_name: String::from("somefile"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
            channels: 2,
            log_level: LogLevel::Info,
        };

        let args_2 = Args {
            file_name: String::from("somefile.wav"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
            channels: 2,
            log_level: LogLevel::Info,
        };

        assert_eq!(args_1.file_name(), args_2.file_name());
    }

    #[test]
    fn test_log_level_returns_correct_level_filter() {
        let error_level_args = Args {
            file_name: String::from("somefile"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
            channels: 2,
            log_level: LogLevel::Error,
        };

        let warn_level_args = Args {
            file_name: String::from("somefile"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
            channels: 2,
            log_level: LogLevel::Warn,
        };

        let info_level_args = Args {
            file_name: String::from("somefile"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
            channels: 2,
            log_level: LogLevel::Info,
        };

        let debug_level_args = Args {
            file_name: String::from("somefile"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
            channels: 2,
            log_level: LogLevel::Debug,
        };

        let trace_level_args = Args {
            file_name: String::from("somefile"),
            format: SampleFormat::Int16,
            sample_rate: 44100,
            channels: 2,
            log_level: LogLevel::Trace,
        };

        assert_eq!(error_level_args.log_level(), LevelFilter::Error);
        assert_eq!(warn_level_args.log_level(), LevelFilter::Warn);
        assert_eq!(info_level_args.log_level(), LevelFilter::Info);
        assert_eq!(debug_level_args.log_level(), LevelFilter::Debug);
        assert_eq!(trace_level_args.log_level(), LevelFilter::Trace);
    }
}
