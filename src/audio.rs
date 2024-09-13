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
    pub fn new(sample_rate: u32, num_channels: u8, format: SampleFormat) -> Self {
        Self {
            sample_rate,
            num_channels,
            format,
        }
    }
}

pub trait AudioLoopback {
    fn new(format: Arc<AudioFormatInfo>) -> Self;
    fn init(&self) -> Nothing;
    fn capture(&self, transmitter: Sender<Vec<u8>>) -> Nothing;
}
