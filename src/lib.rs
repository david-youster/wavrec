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

use audio::{
    sys::LoopbackRecorder, AudioDataMessage, AudioFormatInfo, AudioLoopback,
    RequestedAudioFormatInfo,
};
use cli::Args;
use log::{error, info};
use std::{
    error::Error,
    fmt::Display,
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

#[derive(Debug)]
struct AppError {
    message: String,
}

impl Error for AppError {}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Run the application.
///
/// This will spawn a thread which will pull data from the default audio device and write it to a
/// WAV file. See the [`cli::Args`] struct for options.
///
/// The application will only capture data while there is audio playing. When the audio device is
/// not in use, nothing will be captured.
pub fn run(args: Args) -> Nothing {
    let is_running = Arc::new(AtomicBool::new(true));
    let (audio_transmitter, audio_receiver): (
        Sender<AudioDataMessage>,
        Receiver<AudioDataMessage>,
    ) = mpsc::channel();

    let requested_format = RequestedAudioFormatInfo {
        sample_rate: args.sample_rate,
        num_channels: args.channels,
        format: args.format,
    };

    let loopback_stream: Arc<dyn AudioLoopback> =
        Arc::new(LoopbackRecorder::create(requested_format)?);
    let audio_format = loopback_stream.get_audio_format();
    info!("Loopback recorder initialized with format: {audio_format}");

    setup_terminate_handler(Arc::clone(&is_running))?;
    run_audio_thread(audio_transmitter, Arc::clone(&loopback_stream));
    run_processing_loop(&args.file_name(), audio_receiver, audio_format, is_running)?;

    Ok(())
}

/// Initializes the Ctrl-C handler.
fn setup_terminate_handler(is_running_flag: Arc<AtomicBool>) -> Nothing {
    let result = ctrlc::set_handler(move || {
        info!("Shutting down");
        is_running_flag.store(false, Ordering::Relaxed);
    });

    if result.is_err() {
        return Err(Box::new(AppError {
            message: String::from("Failed to set Ctrl-C handler"),
        }));
    };

    Ok(())
}

/// Initializes the audio thread.
///
/// This thread will run in the background, and continuously send data to the provided
/// [`transmitter`](std::sync::mpsc::Sender), when the audio device is in use.
fn run_audio_thread(
    transmitter: Sender<AudioDataMessage>,
    loopback_stream: Arc<dyn AudioLoopback>,
) {
    info!("Starting audio thread");
    thread::spawn(move || {
        let _ = loopback_stream.capture(transmitter);
    });
}

/// Handles the audio data received from the audio thread.
///
/// Audio data received will be written to the WAV file requested in the [CLI args](cli::Args).
fn run_processing_loop(
    file_name: &str,
    receiver: Receiver<AudioDataMessage>,
    format: AudioFormatInfo,
    is_running: Arc<AtomicBool>,
) -> Nothing {
    info!("Starting processing loop");
    // Handle the captured data sent from the audio thread
    let mut file_writer = WaveWriter::open(file_name, format)?;
    while is_running.load(Ordering::Relaxed) {
        let _ = receiver.try_recv().map(|chunk| match chunk {
            AudioDataMessage::AudioData(chunk) => file_writer.write(chunk),
            AudioDataMessage::Error(err) => {
                error!("Error while writing WAV file: {err}");
                is_running.store(false, Ordering::Relaxed);
                Ok(())
            }
        });
    }
    info!("Creating file: {file_name}");
    file_writer.commit()?;
    file_writer.close()?;
    Ok(())
}
