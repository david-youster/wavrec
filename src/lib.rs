mod audio;
mod wave;

use audio::{sys::LoopbackRecorder, AudioFormatInfo, AudioLoopback, SampleFormat};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
};
use wave::WaveFile;

type Res<T> = Result<T, Box<dyn Error>>;
type Nothing = Res<()>;

const BIT_DEPTH: u8 = 16;
const SAMPLE_RATE: u32 = 44100;
const NUM_CHANNELS: u8 = 2;

struct WaveWriter {
    buffered_writer: BufWriter<File>,
    file_name: String,
    tmp_file_name: String,
    bytes_written: usize,
}

impl WaveWriter {
    pub fn open(file_name: &str) -> Res<Self> {
        let tmp_file_name = file_name.to_string() + ".tmp";
        let file = File::create(&tmp_file_name)?;
        let buffered_writer = BufWriter::new(file);
        let bytes_written = 0;
        let file_name = file_name.to_owned();
        Ok(Self {
            buffered_writer,
            file_name,
            tmp_file_name,
            bytes_written,
        })
    }

    pub fn write(&mut self, data: Vec<u8>) -> Nothing {
        // TODO - needs to handle audio format
        let audio_data: Vec<i16> = data
            .chunks_exact(2)
            .into_iter()
            .map(|s| i16::from_ne_bytes([s[0], s[1]]))
            .collect();

        unsafe {
            self.bytes_written += self.buffered_writer.write(audio_data.align_to::<u8>().1)?;
        }
        Ok(())
    }

    pub fn commit(&mut self) -> Nothing {
        self.buffered_writer.flush()?;
        let mut data = Vec::new();
        File::open(&self.tmp_file_name)?.read_to_end(&mut data)?;

        let format = AudioFormatInfo::new(SAMPLE_RATE, NUM_CHANNELS, SampleFormat::Int16);
        let wav = WaveFile::create(data, format)?;
        wav.write(&self.file_name)?;
        Ok(())
    }
}

impl Drop for WaveWriter {
    fn drop(&mut self) {
        self.buffered_writer.flush().unwrap();

        if Path::new(&self.tmp_file_name).exists() {
            fs::remove_file(&self.tmp_file_name).unwrap();
        }
    }
}

pub fn run() -> Nothing {
    let is_running = Arc::new(AtomicBool::new(true));
    let (audio_transmitter, audio_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    setup_terminate_handler(Arc::clone(&is_running));
    run_audio_thread(audio_transmitter);
    run_processing_loop(audio_receiver, Arc::clone(&is_running))?;

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
fn run_audio_thread(transmitter: Sender<Vec<u8>>) {
    thread::spawn(|| {
        let loopback_stream = LoopbackRecorder::new(BIT_DEPTH, SAMPLE_RATE, NUM_CHANNELS);
        loopback_stream.init().unwrap();
        loopback_stream.capture(transmitter).unwrap();
    });
}

fn run_processing_loop(receiver: Receiver<Vec<u8>>, is_running: Arc<AtomicBool>) -> Nothing {
    // Handle the captured data sent from the audio thread
    let mut file_writer = WaveWriter::open("./wavdata.wav")?;
    while is_running.load(Ordering::Relaxed) {
        let _ = receiver.try_recv().map(|chunk| {
            // TODO - write failure should be handled
            file_writer.write(chunk).unwrap();
        });
    }
    file_writer.commit()?;
    Ok(())
}
