use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::Path,
    sync::Arc,
};

use uuid::Uuid;

use crate::{
    audio::{AudioFormatInfo, SampleFormat},
    Nothing, Res, NUM_CHANNELS, SAMPLE_RATE,
};

type TwoByteField = [u8; 2];
type FourByteField = [u8; 4];

// http://www.ringthis.com/dev/wave_format.htm
struct WaveHeader {
    // RIFF marker
    file_description_header: FourByteField,

    // File size less the 4 bytes of the RIFF marker,
    // and the 4 bytes of this field
    file_size: FourByteField,

    // WAVE description header
    wave_description_header: FourByteField,

    // fmt description - 'fmt' string plus null character
    fmt_description: FourByteField,

    // Size of WAVE description chunk (2 bytes) -> 16
    wave_description_chunk_size: FourByteField,

    // PCM = 1
    type_format: TwoByteField,

    // Mono/Stereo flag
    num_channels: TwoByteField,

    sample_rate: FourByteField,

    // Bit depth / 8 * sample rate
    bytes_per_second: FourByteField,

    // Num channels * bytes per sample
    block_alignment: TwoByteField,

    bit_depth: TwoByteField,
}

impl WaveHeader {
    fn as_bytes(&self) -> Vec<u8> {
        const BYTES_IN_HEADER: usize = 44;
        let mut data: Vec<u8> = Vec::with_capacity(BYTES_IN_HEADER);
        data.extend_from_slice(&self.file_description_header);
        data.extend_from_slice(&self.file_size);
        data.extend_from_slice(&self.wave_description_header);
        data.extend_from_slice(&self.fmt_description);
        data.extend_from_slice(&self.wave_description_chunk_size);
        data.extend_from_slice(&self.type_format);
        data.extend_from_slice(&self.num_channels);
        data.extend_from_slice(&self.sample_rate);
        data.extend_from_slice(&self.bytes_per_second);
        data.extend_from_slice(&self.block_alignment);
        data.extend_from_slice(&self.bit_depth);
        data
    }
}
struct WaveData {
    // ASCII text 'data'
    data_header: FourByteField,

    size_in_bytes: FourByteField,

    data: Vec<u8>,
}

impl WaveData {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(8 + self.data.len());
        data.extend_from_slice(&self.data_header);
        data.extend_from_slice(&self.size_in_bytes);
        data.extend(&self.data);
        data
    }
}

pub struct WaveFile {
    header: WaveHeader,
    data: WaveData,
}

impl WaveFile {
    pub fn create(data: Vec<u8>, format: Arc<AudioFormatInfo>) -> Res<Self> {
        let header = WaveFile::create_header_section(&data, format.as_ref())?;
        let data = WaveFile::create_data_section(data)?;
        Ok(WaveFile { header, data })
    }

    pub fn write(&self, file_name: &str) -> Nothing {
        let header_bytes = self.header.as_bytes();
        let data_bytes = self.data.as_bytes();
        let mut file = File::create(file_name)?;
        file.write_all(&header_bytes)?;
        file.write_all(&data_bytes)?;
        Ok(())
    }

    fn create_header_section(data: &[u8], format: &AudioFormatInfo) -> Res<WaveHeader> {
        // RIFF
        let file_description_header = b"RIFF".to_owned();

        let file_size: FourByteField = ((data.len() + (44 - 8)) as u32).to_le_bytes();

        // WAVE
        let wave_description_header = b"WAVE".to_owned();

        // fmt\0
        let fmt_description = b"fmt ".to_owned();

        // TODO - should be taken from the format
        let wave_description_chunk_size = 16u32.to_le_bytes().to_owned();
        // PCM header - http://bass.radio42.com/help/html/56c44e65-9b99-fa0d-d74a-3d9de3b01e89.htm
        let type_format = match format.format {
            SampleFormat::Int16 | SampleFormat::Int24 | SampleFormat::Int32 => 1u16.to_le_bytes(),
            SampleFormat::Float32 => 3u16.to_le_bytes(),
        };
        let num_channels = (format.num_channels as u16).to_le_bytes();
        let sample_rate = format.sample_rate.to_le_bytes();
        let bytes_per_second =
            ((format.sample_rate * format.format.bit_depth() as u32 * format.num_channels as u32)
                / 8)
            .to_le_bytes();
        let block_alignment =
            (((format.format.bit_depth() * format.num_channels) / 8) as u16).to_le_bytes();
        let bit_depth: TwoByteField = (format.format.bit_depth() as u16).to_le_bytes();

        Ok(WaveHeader {
            file_description_header,
            file_size,
            wave_description_header,
            fmt_description,
            wave_description_chunk_size,
            type_format,
            num_channels,
            sample_rate,
            bytes_per_second,
            block_alignment,
            bit_depth,
        })
    }

    fn create_data_section(data: Vec<u8>) -> Res<WaveData> {
        let data_header = b"data".to_owned();
        let size_in_bytes: FourByteField = (data.len() as u32).to_le_bytes().to_owned();
        Ok(WaveData {
            data_header,
            size_in_bytes,
            data,
        })
    }
}

pub struct WaveWriter {
    buffered_writer: BufWriter<File>,
    file_name: String,
    tmp_file_name: String,
    bytes_written: usize,
    audio_format_info: Arc<AudioFormatInfo>,
}

impl WaveWriter {
    pub fn open(file_name: &str, audio_format_info: Arc<AudioFormatInfo>) -> Res<Self> {
        let mut tmp_dir = env::temp_dir();
        let tmp_file_id = Uuid::new_v4().to_string();
        let tmp_file_name = format!("wavdata-{}", tmp_file_id);
        tmp_dir.push(&tmp_file_name);

        let file = File::create(&tmp_dir)?;
        let buffered_writer = BufWriter::new(file);
        let bytes_written = 0;
        let file_name = file_name.to_owned();
        Ok(Self {
            buffered_writer,
            file_name,
            tmp_file_name: tmp_dir.to_str().unwrap().to_owned(),
            bytes_written,
            audio_format_info,
        })
    }

    pub fn write(&mut self, data: Vec<u8>) -> Nothing {
        self.bytes_written += self.buffered_writer.write(&data)?;
        Ok(())
    }

    pub fn commit(&mut self) -> Nothing {
        self.buffered_writer.flush()?;
        let mut data = Vec::new();
        File::open(&self.tmp_file_name)?.read_to_end(&mut data)?;

        let wav = WaveFile::create(data, Arc::clone(&self.audio_format_info))?;
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
