use std::sync::{mpsc::Sender, Arc};

use clap::ValueEnum;

use crate::Nothing;

pub mod sys;

#[derive(ValueEnum, Clone)]
pub enum SampleFormat {
    Int16,
    Int24,
    Int32,
    Float32,
}

impl SampleFormat {
    pub fn bit_depth(&self) -> u8 {
        match self {
            SampleFormat::Int16 => 16,
            SampleFormat::Int24 => 24,
            SampleFormat::Int32 => 32,
            SampleFormat::Float32 => 32,
        }
    }
}

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
        match self.format {
            SampleFormat::Int16 | SampleFormat::Int24 | SampleFormat::Int32 => 1u16,
            SampleFormat::Float32 => 3u16,
        }
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

pub trait AudioLoopback {
    fn new(format: Arc<AudioFormatInfo>) -> Self;
    fn init(&self) -> Nothing;
    fn capture(&self, transmitter: Sender<Vec<u8>>) -> Nothing;
}
