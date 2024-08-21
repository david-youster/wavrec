use std::sync::mpsc::Sender;
use std::{collections::VecDeque, error::Error, fmt::Display};

use wasapi::{AudioClient, Device, Direction, SampleType, ShareMode, WaveFormat};

use crate::{Nothing, Res};

use crate::audio::AudioLoopback;

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

pub struct WasapiLoopbackRecorder {
    format: WaveFormat,
    chunk_size: usize,
}

impl WasapiLoopbackRecorder {
    fn get_rendering_device(&self) -> Res<Device> {
        wasapi::get_default_device(&Direction::Render)
    }

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
    fn new(bit_depth: u8, sample_rate: u32, num_channels: u8) -> WasapiLoopbackRecorder {
        let format = WaveFormat::new(
            bit_depth as usize,
            bit_depth as usize,
            &SampleType::Int,
            sample_rate as usize,
            num_channels as usize,
            None,
        );
        let chunk_size = 4096;
        WasapiLoopbackRecorder { format, chunk_size }
    }

    fn init(&self) -> Nothing {
        match wasapi::initialize_mta().ok() {
            Ok(_) => Ok(()),
            Err(_) => Err(Box::new(WasapiError::InitMtaFailure)),
        }
    }

    fn capture(&self, transmitter: Sender<Vec<u8>>) -> Nothing {
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
                client.stop_stream()?;
                break;
            }
        }
        Ok(())
    }
}
