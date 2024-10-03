use std::borrow::Borrow;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::{collections::VecDeque, error::Error, fmt::Display};

use log::{debug, error};
use wasapi::{AudioClient, Device, Direction, SampleType, ShareMode, WaveFormat};

use crate::{Nothing, Res};

use crate::audio::{AudioFormatInfo, AudioLoopback, RequestedAudioFormatInfo, SampleFormat};

const TIMEOUT: u32 = 1000000;

#[derive(Debug)]
enum WasapiError {
    InitMtaFailure,
    InvalidBitDepth,
}

impl Error for WasapiError {}

impl Display for WasapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            WasapiError::InitMtaFailure => "Failed to initialize WASAPI MTA",
            WasapiError::InvalidBitDepth => "Invalid bit depth requested",
        };
        write!(f, "{}", message)
    }
}

/// Loopback recorder for Windows.
pub struct WasapiLoopbackRecorder {
    pub audio_format: AudioFormatInfo,

    wasapi_format: WaveFormat,

    /// Numer of audio blocks to send to the [`transmitter`](std::sync::mpsc::Sender) in a single
    /// write.
    chunk_size: usize,

    /// WASAPI [`AudioClient`] for the rendering device,
    client: AudioClient,
}

impl AudioLoopback for WasapiLoopbackRecorder {
    /// Create a new WASAPI-based [`AudioLoopback`] recorder.
    #[allow(refining_impl_trait)]
    fn create(format: RequestedAudioFormatInfo) -> Res<WasapiLoopbackRecorder> {
        debug!("Initializing WASAPI");
        if let Err(_) = wasapi::initialize_mta().ok() {
            return Err(Box::new(WasapiError::InitMtaFailure));
        };

        let rendering_device = wasapi::get_default_device(&Direction::Render)?;
        let mut client = rendering_device.get_iaudioclient()?;

        let default_format = client.get_mixformat()?;
        let bit_depth = format
            .bit_depth()
            .map(|b| b as u8)
            .or(Some(default_format.get_bitspersample() as u8))
            .unwrap();
        let sample_rate = format
            .sample_rate
            .map(|s| s as u32)
            .or(Some(default_format.get_samplespersec()))
            .unwrap();
        let num_channels = format
            .num_channels
            .map(|c| c as u8)
            .or(Some(default_format.get_nchannels() as u8))
            .unwrap();

        let sample_type = match format.format {
            Some(SampleFormat::Int16) | Some(SampleFormat::Int24) | Some(SampleFormat::Int32) => {
                &SampleType::Int
            }
            Some(SampleFormat::Float32) => &SampleType::Float,
            _ => &default_format.get_subformat()?,
        };

        let audio_format = AudioFormatInfo {
            sample_rate,
            num_channels,
            format: match sample_type {
                SampleType::Float => SampleFormat::Float32,
                SampleType::Int => match bit_depth {
                    16 => SampleFormat::Int16,
                    24 => SampleFormat::Int24,
                    32 => SampleFormat::Int32,
                    _ => return Err(Box::new(WasapiError::InvalidBitDepth)),
                },
            },
        };

        let wasapi_format = WaveFormat::new(
            bit_depth as usize,
            bit_depth as usize,
            sample_type,
            sample_rate as usize,
            num_channels as usize,
            None,
        );

        let (_, min_time) = client.get_periods()?;
        client.initialize_client(
            &wasapi_format,
            min_time,
            &Direction::Capture,
            &ShareMode::Shared,
            true,
        )?;

        let chunk_size = 4096;
        Ok(WasapiLoopbackRecorder {
            audio_format,
            wasapi_format,
            chunk_size,
            client,
        })
    }

    /// Capture audio from the loopback stream.
    fn capture(&self, transmitter: Sender<Vec<u8>>) -> Nothing {
        debug!("Preparing WASAPI loopback capture");

        let block_align = self.wasapi_format.get_blockalign();
        let buffer_frame_count = self.client.get_bufferframecount()?;

        let capture_client = self.client.get_audiocaptureclient()?;
        let event_handle = self.client.set_get_eventhandle()?;

        let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
            100 * block_align as usize * (1024 + 2 * buffer_frame_count as usize),
        );
        self.client.start_stream()?;

        loop {
            while sample_queue.len() > block_align as usize * self.chunk_size {
                let mut chunk = vec![0u8; block_align as usize * self.chunk_size];
                for e in chunk.iter_mut() {
                    *e = sample_queue.pop_front().unwrap();
                }
                transmitter.send(chunk)?
            }

            capture_client.read_from_device_to_deque(&mut sample_queue)?;
            if event_handle.wait_for_event(TIMEOUT).is_err() {
                error!("WASAPI timed out waiting for next audio event");
                self.client.stop_stream()?;
                break;
            }
        }
        Ok(())
    }
}
