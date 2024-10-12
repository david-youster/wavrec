use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::Path,
};

use log::{debug, trace};
use uuid::Uuid;

use crate::{audio::AudioFormatInfo, Nothing, Res};

type TwoByteField = [u8; 2];
type FourByteField = [u8; 4];

/// Represents the content of the header section of the WAVE file format.
/// Some resources describing the file format (last accessed 16/09/24):
/// - <http://www.ringthis.com/dev/wave_format.htm>
/// - <http://soundfile.sapp.org/doc/WaveFormat>
struct WaveHeader {
    /// This will always be the value `RIFF`.
    file_description_header: FourByteField,

    // File size less the 4 bytes of the RIFF marker, and the 4 bytes of this field.
    file_size: FourByteField,

    /// This will always be the value `WAVE`.
    wave_description_header: FourByteField,

    /// This will always be the value `fmt `. Note the space at the end.
    fmt_description: FourByteField,

    /// This is the size in bytes of the type format, channels, sample rate, bytes per second, block
    /// alignment and bit depth sections.
    wave_description_chunk_size: FourByteField,

    /// For PCM (integer audio), use `1`. For floating point audio, use `3`.
    type_format: TwoByteField,

    /// Number of audio channels. Channel audio will be interleaved.
    num_channels: TwoByteField,

    /// The sample rate.
    sample_rate: FourByteField,

    /// Number of bytes per second in the audio.
    /// `(Sample rate * bit depth) / 8`
    bytes_per_second: FourByteField,

    /// Number of bytes per audio frame.
    /// `Number of channels * bit depth / 8`
    block_alignment: TwoByteField,

    /// Audio bit depth.
    bit_depth: TwoByteField,
}

impl WaveHeader {
    const BYTES_IN_HEADER: usize = 44;

    /// Create a new [`WaveHeader`] based on the given [`AudioFormatInfo`] and data size.
    fn create(format: AudioFormatInfo, data_size: usize) -> Res<WaveHeader> {
        trace!("Preparing WAV header data");
        let file_description_header = b"RIFF".to_owned();
        let file_size: FourByteField =
            ((data_size + (Self::BYTES_IN_HEADER - 8)) as u32).to_le_bytes();
        let wave_description_header = b"WAVE".to_owned();
        let fmt_description = b"fmt ".to_owned();
        let wave_description_chunk_size = 16u32.to_le_bytes().to_owned();
        let type_format = format.type_format_header().to_le_bytes();
        let num_channels = (format.num_channels as u16).to_le_bytes();
        let sample_rate = format.sample_rate.to_le_bytes();
        let bytes_per_second = format.bytes_per_second().to_le_bytes();
        let block_alignment = format.block_alignment().to_le_bytes();
        let bit_depth: TwoByteField = (format.bit_depth() as u16).to_le_bytes();

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

    /// Build the formatted WAV file header, ready for writing.
    fn as_bytes(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::with_capacity(Self::BYTES_IN_HEADER);
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

/// Represents the data section of a WAV file, including the 'data' header.
struct WaveData {
    data_header: FourByteField,
    size_in_bytes: FourByteField,
    data: Vec<u8>,
}

impl WaveData {
    /// Create the data section of the WAV file.
    fn create(data: Vec<u8>) -> Res<WaveData> {
        trace!("Preparing WAV data section");
        let data_header = b"data".to_owned();
        let size_in_bytes: FourByteField = (data.len() as u32).to_le_bytes().to_owned();
        Ok(WaveData {
            data_header,
            size_in_bytes,
            data,
        })
    }
    /// Return the formatted bytes in the data section, ready for writing.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(8 + self.data.len());
        data.extend_from_slice(&self.data_header);
        data.extend_from_slice(&self.size_in_bytes);
        data.extend(&self.data);
        data
    }
}

/// Represents a complete WAV file, separated into header and data sections. The `header` and
/// `data` properties should contain everything necessary to write a valid WAV file.
pub struct WaveFile {
    header: WaveHeader,
    data: WaveData,
}

impl WaveFile {
    /// Prepare the data for a new WAV file.
    pub fn create(data: Vec<u8>, format: AudioFormatInfo) -> Res<Self> {
        debug!("Preparing WAV file data");
        let header = WaveHeader::create(format, data.len())?;
        let data = WaveData::create(data)?;
        Ok(WaveFile { header, data })
    }

    /// Write the WAV data to file.
    pub fn write(&self, file_name: &str) -> Nothing {
        debug!("Writing to file: {file_name}");
        let header_bytes = self.header.as_bytes();
        let data_bytes = self.data.as_bytes();
        let mut file = File::create(file_name)?;
        file.write_all(&header_bytes)?;
        file.write_all(&data_bytes)?;
        Ok(())
    }
}

/// Buffered WAV file writer. Opening a WAV file allows writing to a buffer, which can later be
/// written to disk.
///
/// To use, a writer should be opened, written to, committed and closed.
pub struct WaveWriter {
    buffered_writer: BufWriter<File>,
    file_name: String,
    tmp_file_name: String,
    bytes_written: usize,
    audio_format_info: AudioFormatInfo,
}

impl WaveWriter {
    /// Prepares a new WaveWriter for writing audio data to disk.
    ///
    /// This uses a temporary file as a data buffer, which will later be written to a correctly
    /// formatted WAV file, when the [`WaveWriter::commit`] method is called.
    pub fn open(file_name: &str, audio_format_info: AudioFormatInfo) -> Res<Self> {
        let mut tmp_dir = env::temp_dir();
        let tmp_file_id = Uuid::new_v4().to_string();
        let tmp_file_name = format!("wavdata-{}", tmp_file_id);
        tmp_dir.push(&tmp_file_name);

        debug!("Creating temporary file: {tmp_file_name}");

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

    /// Write a chunk of data to the buffer. Audio data should be appropriately formatted.
    pub fn write(&mut self, data: Vec<u8>) -> Nothing {
        self.bytes_written += self.buffered_writer.write(&data)?;
        Ok(())
    }

    /// Commit the written audio data to disk
    pub fn commit(&mut self) -> Nothing {
        debug!("Preparing to write from temp file to WAV file");
        self.buffered_writer.flush()?;
        let mut data = Vec::new();
        File::open(&self.tmp_file_name)?.read_to_end(&mut data)?;

        let wav = WaveFile::create(data, self.audio_format_info)?;
        wav.write(&self.file_name)?;
        Ok(())
    }

    /// Clean up the temporary file used by the [`BufWriter`].
    pub fn close(self) -> Nothing {
        debug!("Removing temporary file");
        if Path::new(&self.tmp_file_name).exists() {
            fs::remove_file(&self.tmp_file_name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::audio::SampleFormat;

    use super::*;

    #[test]
    fn test_create_wave_header_returns_wave_data_with_correct_static_fields() {
        let header = create_wave_header(44100, SampleFormat::Int16, 2, 0);
        assert_eq!(header.file_description_header, *b"RIFF");
        assert_eq!(header.fmt_description, *b"fmt ");
        assert_eq!(header.wave_description_header, *b"WAVE");
        assert_eq!(u32::from_le_bytes(header.wave_description_chunk_size), 16);
    }

    #[test]
    fn test_create_wave_header_returns_wave_data_with_correct_calculated_fields() {
        validate_wave_header_fields(44100, SampleFormat::Int16, 2, 0);
        validate_wave_header_fields(44100, SampleFormat::Int16, 2, 100);
        validate_wave_header_fields(48000, SampleFormat::Int16, 2, 100);
        validate_wave_header_fields(96000, SampleFormat::Int16, 2, 100);

        validate_wave_header_fields(44100, SampleFormat::Int24, 2, 0);
        validate_wave_header_fields(44100, SampleFormat::Int24, 2, 100);
        validate_wave_header_fields(48000, SampleFormat::Int24, 2, 100);
        validate_wave_header_fields(96000, SampleFormat::Int24, 2, 100);

        validate_wave_header_fields(44100, SampleFormat::Int32, 2, 0);
        validate_wave_header_fields(44100, SampleFormat::Int32, 2, 100);
        validate_wave_header_fields(48000, SampleFormat::Int32, 2, 100);
        validate_wave_header_fields(96000, SampleFormat::Int32, 2, 100);

        validate_wave_header_fields(44100, SampleFormat::Float32, 2, 0);
        validate_wave_header_fields(44100, SampleFormat::Float32, 2, 100);
        validate_wave_header_fields(48000, SampleFormat::Float32, 2, 100);
        validate_wave_header_fields(96000, SampleFormat::Float32, 2, 100);
    }

    #[test]
    fn test_wave_header_bytes_contain_correct_static_data() {
        let header = create_wave_header(44100, SampleFormat::Int16, 2, 0).as_bytes();
        assert_eq!(header[0..4], *b"RIFF");
        assert_eq!(header[8..12], *b"WAVE");
        assert_eq!(header[12..16], *b"fmt ");

        // Wave description chunk size
        assert_eq!(header[16..20], 16u32.to_le_bytes());
    }

    #[test]
    fn test_wave_header_bytes_contain_correct_calculated_data() {
        validate_wave_header_bytes(44100, SampleFormat::Int16, 2, 0);
        validate_wave_header_bytes(44100, SampleFormat::Int16, 2, 100);
        validate_wave_header_bytes(48000, SampleFormat::Int16, 2, 100);
        validate_wave_header_bytes(96000, SampleFormat::Int16, 2, 100);

        validate_wave_header_bytes(44100, SampleFormat::Int24, 2, 0);
        validate_wave_header_bytes(44100, SampleFormat::Int24, 2, 100);
        validate_wave_header_bytes(48000, SampleFormat::Int24, 2, 100);
        validate_wave_header_bytes(96000, SampleFormat::Int24, 2, 100);

        validate_wave_header_bytes(44100, SampleFormat::Int32, 2, 0);
        validate_wave_header_bytes(44100, SampleFormat::Int32, 2, 100);
        validate_wave_header_bytes(48000, SampleFormat::Int32, 2, 100);
        validate_wave_header_bytes(96000, SampleFormat::Int32, 2, 100);

        validate_wave_header_bytes(44100, SampleFormat::Float32, 2, 0);
        validate_wave_header_bytes(44100, SampleFormat::Float32, 2, 100);
        validate_wave_header_bytes(48000, SampleFormat::Float32, 2, 100);
        validate_wave_header_bytes(96000, SampleFormat::Float32, 2, 100);
    }

    #[test]
    fn test_wave_data_contains_correct_static_data() {
        let data = WaveData::create(vec![]).unwrap();
        assert_eq!(data.data_header, *b"data");
    }

    #[test]
    fn test_wave_data_conains_correct_size() {
        let data = WaveData::create(vec![]).unwrap();
        assert_eq!(data.size_in_bytes, 0u32.to_le_bytes());

        let data = WaveData::create(vec![0u8; 100]).unwrap();
        assert_eq!(data.size_in_bytes, 100u32.to_le_bytes());
    }

    #[test]
    fn test_wave_data_contains_correct_data() {
        let data = WaveData::create(vec![]).unwrap();
        assert_eq!(data.data, vec![]);

        let values: Vec<u8> = vec![1, 2, 3, 4];
        let data = WaveData::create(values.clone()).unwrap();
        assert_eq!(data.data, values);
    }

    #[test]
    fn test_wave_data_bytes_contains_correct_static_data() {
        let data = WaveData::create(vec![]).unwrap().as_bytes();
        assert_eq!(data[0..4], *b"data");
    }

    #[test]
    fn test_wave_data_bytes_contains_correct_size() {
        let data = WaveData::create(vec![]).unwrap().as_bytes();
        assert_eq!(data[4..8], 0u32.to_le_bytes());

        let data = WaveData::create(vec![1, 2, 3, 4]).unwrap().as_bytes();
        assert_eq!(data[4..8], 4u32.to_le_bytes());
    }

    #[test]
    fn test_wave_data_bytes_contains_correct_data() {
        let data = WaveData::create(vec![]).unwrap().as_bytes();
        assert_eq!(data[8..], vec![]);

        let values: Vec<u8> = vec![1, 2, 3, 4];
        let data = WaveData::create(values.clone()).unwrap().as_bytes();
        assert_eq!(data[8..], values);
    }

    fn create_wave_header(
        sample_rate: u32,
        format: SampleFormat,
        num_channels: u8,
        data_size: usize,
    ) -> WaveHeader {
        let format = AudioFormatInfo {
            sample_rate,
            num_channels,
            format,
        };
        WaveHeader::create(format, data_size).unwrap()
    }

    fn validate_wave_header_fields(
        sample_rate: u32,
        format: SampleFormat,
        num_channels: u8,
        data_size: usize,
    ) {
        let header = create_wave_header(sample_rate, format, num_channels, data_size);
        assert_eq!(
            u32::from_le_bytes(header.file_size),
            (data_size + WaveHeader::BYTES_IN_HEADER - 8)
                .try_into()
                .unwrap()
        );

        assert_eq!(
            u16::from_le_bytes(header.type_format),
            format.type_format_header()
        );
        assert_eq!(u16::from_le_bytes(header.num_channels), num_channels.into());
        assert_eq!(u32::from_le_bytes(header.sample_rate), sample_rate);
        assert_eq!(
            u32::from_le_bytes(header.bytes_per_second),
            (sample_rate * format.bit_depth() as u32 * num_channels as u32) / 8
        );

        assert_eq!(
            u16::from_le_bytes(header.block_alignment),
            ((num_channels * format.bit_depth()) / 8).into()
        );

        assert_eq!(
            u16::from_le_bytes(header.bit_depth),
            format.bit_depth().into()
        );
    }

    fn validate_wave_header_bytes(
        sample_rate: u32,
        format: SampleFormat,
        num_channels: u8,
        data_size: usize,
    ) {
        let header = create_wave_header(sample_rate, format, num_channels, data_size).as_bytes();

        assert_eq!(
            header[4..8],
            ((data_size + WaveHeader::BYTES_IN_HEADER - 8) as u32).to_le_bytes()
        );

        assert_eq!(header[20..22], format.type_format_header().to_le_bytes());
        assert_eq!(header[22..24], (num_channels as u16).to_le_bytes());
        assert_eq!(header[24..28], sample_rate.to_le_bytes());

        // Bytes per second
        assert_eq!(
            header[28..32],
            ((sample_rate * format.bit_depth() as u32 * num_channels as u32) / 8).to_le_bytes()
        );

        // Block alignment
        assert_eq!(
            header[32..34],
            (((num_channels * format.bit_depth()) / 8) as u16).to_le_bytes()
        );

        assert_eq!(header[34..36], (format.bit_depth() as u16).to_le_bytes());
    }
}
