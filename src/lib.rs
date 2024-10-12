//! The WAV Recorder library crate exposes a single [`run`] function.
//!
//! Calling this function will begin the audio capture loop in a background thread, and the audio
//! processing loop on the main thread. The processing loop will run until the application is
//! terminated with `Ctrl-C`, at which point the buffered audio data will be written to the final
//! WAV file.
//!
//! Audio format settings and other options can be set by setting the desired values via the
//! [`cli::Args`] parameter.
#[warn(missing_docs)]
mod audio;
pub mod cli;
mod wave;

use audio::{sys::LoopbackRecorder, AudioFormatInfo, AudioLoopback, RequestedAudioFormatInfo};
use cli::Args;
use log::{error, info};
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

/// Run the application.
///
/// This will spawn a thread which will pull data from the default audio device and write it to a
/// WAV file. See the [`cli::Args`] struct for options.
///
/// The application will only capture data while there is audio playing. When the audio device is
/// not in use, nothing will be captured.
///
/// # Panic
/// Panics if the audio processing loop fails.
pub fn run(args: Args) -> Nothing {
    let is_running = Arc::new(AtomicBool::new(true));
    let (audio_transmitter, audio_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    let requested_format = RequestedAudioFormatInfo {
        sample_rate: args.sample_rate,
        num_channels: args.channels,
        format: args.format,
    };

    let loopback_stream: Arc<dyn AudioLoopback> =
        Arc::new(LoopbackRecorder::create(requested_format)?);
    let audio_format = loopback_stream.get_audio_format();
    info!("Loopback recorder initialized with format: {audio_format}");

    setup_terminate_handler(Arc::clone(&is_running));
    run_audio_thread(audio_transmitter, Arc::clone(&loopback_stream));
    run_processing_loop(&args.file_name(), audio_receiver, audio_format, is_running)?;

    Ok(())
}

/// Initializes the Ctrl-C handler.
///
/// # Panic
/// Panics if the [`ctrlc`] crate fails to set the handler
fn setup_terminate_handler(is_running_flag: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        info!("Shutting down");
        is_running_flag.store(false, Ordering::Relaxed);
    })
    .expect("Unable to set Ctrl-C signal handler");
}

/// Initializes the audio thread.
///
/// This thread will run in the background, and continuously send data to the provided
/// [`transmitter`](std::sync::mpsc::Sender), when the audio device is in use.
///
/// # Panic
/// Panics if the [`LoopbackRecorder`] fails for some reason.
fn run_audio_thread(transmitter: Sender<Vec<u8>>, loopback_stream: Arc<dyn AudioLoopback>) {
    info!("Starting audio thread");
    thread::spawn(move || {
        loopback_stream.capture(transmitter).unwrap();
    });
}

/// Handles the audio data received from the audio thread.
///
/// Audio data received will be writted to the WAV file requested in the [CLI args](cli::Args).
fn run_processing_loop(
    file_name: &str,
    receiver: Receiver<Vec<u8>>,
    format: AudioFormatInfo,
    is_running: Arc<AtomicBool>,
) -> Nothing {
    info!("Starting processing loop");
    // Handle the captured data sent from the audio thread
    let mut file_writer = WaveWriter::open(file_name, format)?;
    while is_running.load(Ordering::Relaxed) {
        let _ = receiver.try_recv().map(|chunk| {
            if let Err(err) = file_writer.write(chunk) {
                error!("Error while writing WAV file: {err}");
                is_running.store(false, Ordering::Relaxed);
            }
        });
    }
    info!("Creating file: {file_name}");
    file_writer.commit()?;
    file_writer.close()?;
    Ok(())
}
