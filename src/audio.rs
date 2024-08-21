use std::sync::mpsc::Sender;

use crate::Nothing;

pub mod sys;

pub enum SampleFormat {
    Int16,
    _Int32,
    _Float32,
    _Float64,
}

impl SampleFormat {
    pub fn bit_depth(&self) -> u8 {
        match self {
            SampleFormat::Int16 => 16,
            SampleFormat::_Int32 => 32,
            SampleFormat::_Float32 => 32,
            SampleFormat::_Float64 => 64,
        }
    }
}

pub struct AudioFormatInfo {
    pub sample_rate: u32,
    pub num_channels: u8,
    pub format: SampleFormat,
}

impl AudioFormatInfo {
    pub fn new(sample_rate: u32, num_channels: u8, format: SampleFormat) -> Self {
        Self {
            sample_rate,
            num_channels,
            format,
        }
    }
}

pub trait AudioLoopback {
    fn new(bit_depth: u8, sample_rate: u32, num_channels: u8) -> Self;
    fn init(&self) -> Nothing;
    fn capture(&self, transmitter: Sender<Vec<u8>>) -> Nothing;
}
