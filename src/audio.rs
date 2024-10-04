use std::{
    fmt::Display,
    sync::{mpsc::Sender, Arc},
};

use clap::ValueEnum;

use crate::{Nothing, Res};

pub mod sys;

/// Audio bit depth and sample format.
#[derive(ValueEnum, Clone, Copy)]
pub enum SampleFormat {
    Int16,
    Int24,
    Int32,
    Float32,
}

impl SampleFormat {
    /// Return the appropriaate bit depth for the selected sample format.
    pub fn bit_depth(&self) -> u8 {
        match self {
            SampleFormat::Int16 => 16,
            SampleFormat::Int24 => 24,
            SampleFormat::Int32 => 32,
            SampleFormat::Float32 => 32,
        }
    }

    /// Gives the type format header for the selected audio format. `1` for PCM, `3` for float.
    pub fn type_format_header(&self) -> u16 {
        match self {
            SampleFormat::Int16 | SampleFormat::Int24 | SampleFormat::Int32 => 1u16,
            SampleFormat::Float32 => 3u16,
        }
    }
}

/// Audio format info requested by the user
pub struct RequestedAudioFormatInfo {
    pub sample_rate: Option<u32>,
    pub num_channels: Option<u8>,
    pub format: Option<SampleFormat>,
}

impl RequestedAudioFormatInfo {
    pub fn bit_depth(&self) -> Option<u8> {
        self.format.map(|f| f.bit_depth())
    }
}

/// Basic info about the audio format to capture and write.
#[derive(Copy, Clone)]
pub struct AudioFormatInfo {
    pub sample_rate: u32,
    pub num_channels: u8,
    pub format: SampleFormat,
}

impl AudioFormatInfo {
    /// Return the number of bits per sample for the chosen [`SampleFormat`].
    pub fn bit_depth(&self) -> u8 {
        self.format.bit_depth()
    }

    /// Return the audio type header to the chosen [`SampleFormat`].
    /// `1` should be used for PCM (integer) audio, `3` for floating point.
    pub fn type_format_header(&self) -> u16 {
        self.format.type_format_header()
    }

    /// Return the number of bytes per second based on the given sample rate, bit depth and number
    /// of channels.
    pub fn bytes_per_second(&self) -> u32 {
        (self.sample_rate * self.bit_depth() as u32 * self.num_channels as u32) / 8
    }

    /// Return the block alignment for the audio format.
    /// The block alignment is the number of bytes per audio frame of interleaved audio data.
    /// It's calculated by multiplying the bytes per sample by the number of channels.
    pub fn block_alignment(&self) -> u16 {
        (self.bit_depth() as u16 * self.num_channels as u16) / 8
    }
}

impl Display for AudioFormatInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\nSample Rate: {}\nBit Depth: {}\nNumber of channels: {}\n",
            self.sample_rate,
            self.bit_depth(),
            self.num_channels
        )?;
        write!(
            f,
            "Sample format: {}",
            match self.format {
                SampleFormat::Int16 => "Integer",
                SampleFormat::Int24 => "Integer",
                SampleFormat::Int32 => "Integer",
                SampleFormat::Float32 => "Float",
            }
        )
    }
}

pub trait AudioLoopback: Send + Sync {
    /// Create a new instance of the `AudioLoopback` system.
    fn create(format: RequestedAudioFormatInfo) -> Res<impl AudioLoopback>
    where
        Self: Sized;

    /// Return an [`AudioFormatInfo`] struct with the format info that the loopback recorder has
    /// been configured to use. This will be initialized from the [`RequestedAudioFormatInfo`]
    /// passed to the [`AudioLoopback::create`] method, with missing values being set to the
    /// default for the audio system.
    fn get_audio_format(&self) -> AudioFormatInfo;

    /// Start the audio capture loop. Audio will be written to the [`transmitter`](std::sync::mpsc::Sender).
    fn capture(&self, transmitter: Sender<Vec<u8>>) -> Nothing;
}

#[cfg(test)]
mod tests {

    use super::*;

    const DEFAULT_SAMPLE_RATE: u32 = 44100;
    const DEFAULT_NUM_CHANNELS: u8 = 2;

    #[test]
    fn sample_format_returns_correct_bit_depth_values() {
        assert_eq!(SampleFormat::Int16.bit_depth(), 16);
        assert_eq!(SampleFormat::Int24.bit_depth(), 24);
        assert_eq!(SampleFormat::Int32.bit_depth(), 32);
        assert_eq!(SampleFormat::Float32.bit_depth(), 32);
    }

    #[test]
    fn audio_format_info_returns_correct_bit_depth_values() {
        let format_info = create_audio_format_info(
            DEFAULT_SAMPLE_RATE,
            SampleFormat::Int16,
            DEFAULT_NUM_CHANNELS,
        );
        assert_eq!(format_info.bit_depth(), 16);

        let format_info = create_audio_format_info(
            DEFAULT_SAMPLE_RATE,
            SampleFormat::Int24,
            DEFAULT_NUM_CHANNELS,
        );
        assert_eq!(format_info.bit_depth(), 24);

        let format_info = create_audio_format_info(
            DEFAULT_SAMPLE_RATE,
            SampleFormat::Int32,
            DEFAULT_NUM_CHANNELS,
        );
        assert_eq!(format_info.bit_depth(), 32);

        let format_info = create_audio_format_info(
            DEFAULT_SAMPLE_RATE,
            SampleFormat::Float32,
            DEFAULT_NUM_CHANNELS,
        );
        assert_eq!(format_info.bit_depth(), 32);
    }

    #[test]
    fn audio_format_info_returns_correct_type_format() {
        let format_info = create_audio_format_info(
            DEFAULT_SAMPLE_RATE,
            SampleFormat::Int16,
            DEFAULT_NUM_CHANNELS,
        );
        assert_eq!(format_info.type_format_header(), 0x0001);

        let format_info = create_audio_format_info(
            DEFAULT_SAMPLE_RATE,
            SampleFormat::Int24,
            DEFAULT_NUM_CHANNELS,
        );
        assert_eq!(format_info.type_format_header(), 0x0001);

        let format_info = create_audio_format_info(
            DEFAULT_SAMPLE_RATE,
            SampleFormat::Int32,
            DEFAULT_NUM_CHANNELS,
        );
        assert_eq!(format_info.type_format_header(), 0x0001);

        let format_info = create_audio_format_info(
            DEFAULT_SAMPLE_RATE,
            SampleFormat::Float32,
            DEFAULT_NUM_CHANNELS,
        );
        assert_eq!(format_info.type_format_header(), 0x0003);
    }

    #[test]
    fn audio_format_info_bytes_per_second_returns_correct_value() {
        validate_bytes_per_second(44100, SampleFormat::Int16, DEFAULT_NUM_CHANNELS);
        validate_bytes_per_second(44100, SampleFormat::Int24, DEFAULT_NUM_CHANNELS);
        validate_bytes_per_second(48000, SampleFormat::Int16, DEFAULT_NUM_CHANNELS);
        validate_bytes_per_second(48000, SampleFormat::Int24, DEFAULT_NUM_CHANNELS);
        validate_bytes_per_second(96000, SampleFormat::Int32, DEFAULT_NUM_CHANNELS);
        validate_bytes_per_second(96000, SampleFormat::Float32, DEFAULT_NUM_CHANNELS);
    }

    #[test]
    fn audio_format_info_block_alignment_returns_correct_value() {
        validate_block_alignment(SampleFormat::Int16, DEFAULT_NUM_CHANNELS);
        validate_block_alignment(SampleFormat::Int24, DEFAULT_NUM_CHANNELS);
        validate_block_alignment(SampleFormat::Int32, DEFAULT_NUM_CHANNELS);
        validate_block_alignment(SampleFormat::Float32, DEFAULT_NUM_CHANNELS);

        validate_block_alignment(SampleFormat::Int16, DEFAULT_NUM_CHANNELS * 2);
        validate_block_alignment(SampleFormat::Int24, DEFAULT_NUM_CHANNELS * 2);
        validate_block_alignment(SampleFormat::Int32, DEFAULT_NUM_CHANNELS * 2);
        validate_block_alignment(SampleFormat::Float32, DEFAULT_NUM_CHANNELS * 2);
    }

    fn create_audio_format_info(
        sample_rate: u32,
        sample_format: SampleFormat,
        num_channels: u8,
    ) -> AudioFormatInfo {
        AudioFormatInfo {
            sample_rate,
            num_channels,
            format: sample_format,
        }
    }

    fn validate_bytes_per_second(sample_rate: u32, sample_format: SampleFormat, num_channels: u8) {
        let format_info = create_audio_format_info(sample_rate, sample_format, num_channels);
        let expected_result =
            (sample_rate * sample_format.bit_depth() as u32 * num_channels as u32) / 8;
        assert_eq!(format_info.bytes_per_second(), expected_result);
    }

    fn validate_block_alignment(sample_format: SampleFormat, num_channels: u8) {
        let format_info =
            create_audio_format_info(DEFAULT_SAMPLE_RATE, sample_format, num_channels);
        let expected_result: u16 = (format_info.bit_depth() as u16 * num_channels as u16) / 8;
        assert_eq!(format_info.block_alignment(), expected_result);
    }
}
