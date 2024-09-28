use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::{collections::VecDeque, error::Error, fmt::Display};

use log::{debug, error};
use wasapi::{AudioClient, Device, Direction, SampleType, ShareMode, WaveFormat};

use crate::{Nothing, Res};

use crate::audio::{AudioFormatInfo, AudioLoopback, SampleFormat};

const TIMEOUT: u32 = 1000000;

#[derive(Debug)]
enum WasapiError {
    InitMtaFailure,
}

impl Error for WasapiError {}

impl Display for WasapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            WasapiError::InitMtaFailure => "Failed to initialize WASAPI MTA",
        };
        write!(f, "{}", message)
    }
}

/// Loopback recorder for Windows.
pub struct WasapiLoopbackRecorder {
    format: WaveFormat,

    /// Numer of audio blocks to send to the [`transmitter`](std::sync::mpsc::Sender) in a single
    /// write.
    chunk_size: usize,
}

impl WasapiLoopbackRecorder {
    /// Get the WASAPI rendering device.
    fn get_rendering_device(&self) -> Res<Device> {
        wasapi::get_default_device(&Direction::Render)
    }

    /// Build and initialise the WASAPI [`AudioClient`]
    fn get_audio_client(&self, device: &Device) -> Res<AudioClient> {
        let mut client = device.get_iaudioclient()?;
        let (_, min_time) = client.get_periods()?;
        client.initialize_client(
            &self.format,
            min_time,
            &Direction::Capture,
            &ShareMode::Shared,
            true,
        )?;
        Ok(client)
    }
}

impl AudioLoopback for WasapiLoopbackRecorder {
    /// Create a new WASAPI-based [`AudioLoopback`] recorder.
    fn new(format: Arc<AudioFormatInfo>) -> WasapiLoopbackRecorder {
        let bit_depth = format.format.bit_depth() as usize;
        let sample_rate = format.sample_rate as usize;
        let num_channels = format.num_channels as usize;

        let sample_type = match format.format {
            SampleFormat::Int16 | SampleFormat::Int24 | SampleFormat::Int32 => &SampleType::Int,
            _ => &SampleType::Float,
        };
        let format = WaveFormat::new(
            bit_depth,
            bit_depth,
            sample_type,
            sample_rate,
            num_channels,
            None,
        );
        let chunk_size = 4096;
        WasapiLoopbackRecorder { format, chunk_size }
    }

    /// Initialize the WASAPI system.
    fn init(&self) -> Nothing {
        debug!("Initializing WASAPI");
        match wasapi::initialize_mta().ok() {
            Ok(_) => Ok(()),
            Err(_) => Err(Box::new(WasapiError::InitMtaFailure)),
        }
    }

    /// Capture audio from the loopback stream.
    fn capture(&self, transmitter: Sender<Vec<u8>>) -> Nothing {
        debug!("Preparing WASAPI loopback capture");
        let rendering_device = self.get_rendering_device()?;
        let client: AudioClient = self.get_audio_client(&rendering_device)?;

        let block_align = self.format.get_blockalign();
        let buffer_frame_count = client.get_bufferframecount()?;

        let capture_client = client.get_audiocaptureclient()?;
        let event_handle = client.set_get_eventhandle()?;

        let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
            100 * block_align as usize * (1024 + 2 * buffer_frame_count as usize),
        );
        client.start_stream()?;

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
                client.stop_stream()?;
                break;
            }
        }
        Ok(())
    }
}
