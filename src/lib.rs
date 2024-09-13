mod audio;
pub mod cli;
mod wave;

use audio::{sys::LoopbackRecorder, AudioFormatInfo, AudioLoopback};
use cli::Args;
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
};
use wave::WaveWriter;

type Res<T> = Result<T, Box<dyn Error>>;
type Nothing = Res<()>;

const SAMPLE_RATE: u32 = 44100;
const NUM_CHANNELS: u8 = 2;

pub fn run(args: Args) -> Nothing {
    let is_running = Arc::new(AtomicBool::new(true));
    let (audio_transmitter, audio_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    let audio_format = Arc::new(AudioFormatInfo {
        sample_rate: SAMPLE_RATE,
        num_channels: NUM_CHANNELS,
        format: args.format,
    });
    setup_terminate_handler(Arc::clone(&is_running));
    run_audio_thread(audio_transmitter, Arc::clone(&audio_format));
    run_processing_loop(&args.file_name, audio_receiver, audio_format, is_running)?;

    Ok(())
}

/// Initializes the Ctrl-C handler
///
/// # Panics
/// Panics if the [`ctrlc`] crate fails to set the handler
fn setup_terminate_handler(is_running_flag: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        is_running_flag.store(false, Ordering::Relaxed);
    })
    .expect("Unable to set Ctrl-C signal handler");
}

/// Initializes the audio thread.
fn run_audio_thread(transmitter: Sender<Vec<u8>>, format: Arc<AudioFormatInfo>) {
    thread::spawn(move || {
        let loopback_stream = LoopbackRecorder::new(Arc::clone(&format));
        loopback_stream.init().unwrap();
        loopback_stream.capture(transmitter).unwrap();
    });
}

fn run_processing_loop(
    file_name: &str,
    receiver: Receiver<Vec<u8>>,
    format: Arc<AudioFormatInfo>,
    is_running: Arc<AtomicBool>,
) -> Nothing {
    // Handle the captured data sent from the audio thread
    let mut file_writer = WaveWriter::open(file_name, format)?;
    while is_running.load(Ordering::Relaxed) {
        let _ = receiver.try_recv().map(|chunk| {
            // TODO - write failure should be handled
            file_writer.write(chunk).unwrap();
        });
    }
    file_writer.commit()?;
    Ok(())
}
