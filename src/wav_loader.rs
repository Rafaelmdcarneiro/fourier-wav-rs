use std::io;
use std::io::Cursor;
use std::fmt;
use std::str;
use std::error;
use std::fs::File;
use std::io::prelude::*;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};

const CHUNK_DATA_OFFSET: usize = 8;

#[derive(Debug)]
struct Chunk {
    id: String,
    size: u32,
}

#[derive(Debug)]
pub struct Wav {
    pub num_channels: u16,
    pub sample_rate: u32,
    pub byte_rate: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub data: WavData,
}

#[derive(Debug)]
pub enum WavData {
    Unspecified,
    U8(Vec<u8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
}

#[derive(Debug)]
pub enum WavLoadError {
    Io(io::Error),
    Parse(String),
}

impl error::Error for WavLoadError {
    fn description(&self) -> &str {
        match *self {
            WavLoadError::Io(ref err) => err.description(),
            WavLoadError::Parse(ref err) => err,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            WavLoadError::Io(ref err) => Some(err),
            WavLoadError::Parse(_) => None,
        }
    }
}

impl fmt::Display for WavLoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WavLoadError::Io(ref err) => write!(f, "IO error: {}", err),
            WavLoadError::Parse(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl From<io::Error> for WavLoadError {
    fn from(error: io::Error) -> WavLoadError {
        WavLoadError::Io(error)
    }
}

impl From<String> for WavLoadError {
    fn from(error_msg: String) -> WavLoadError {
        WavLoadError::Parse(error_msg)
    }
}

macro_rules! wav_assert {
    ($a:expr, $msg:expr) => {
        if !$a {
            return Err(WavLoadError::Parse($msg));
        }
    }
}

impl Wav {

    fn new() -> Wav {
        Wav {
            num_channels: 0,
            sample_rate: 0,
            byte_rate: 0,
            block_align: 0,
            bits_per_sample: 0,
            data: WavData::Unspecified,
        }
    }

    pub fn from_file(path: &str) -> Result<Wav, WavLoadError> {
        let mut f = try!(File::open(path));
        let mut buf = Vec::new();
        try!(f.read_to_end(&mut buf));

        Wav::buf_to_wav(&mut buf)
    }

    fn parse_chunk(buf: &[u8]) -> Option<Chunk> {
        if buf.len() < 8 {
            None
        } else {
            let mut chunk_id = str::from_utf8(&buf[0..4]).unwrap().to_string();
            let chunk_size = LittleEndian::read_u32(&buf[4..8]);

            Some(Chunk {
                id: chunk_id,
                size: chunk_size,
            })
        }
    }

    fn parse_fmt_chunk_data(buf: &[u8], wav: &mut Wav) -> Result<(), WavLoadError> {
        let mut reader = Cursor::new(buf);
        let audio_format = try!(reader.read_u16::<LittleEndian>());
        wav_assert!(audio_format == 1, "Audio format is not PCM".to_string());
        wav.num_channels = try!(reader.read_u16::<LittleEndian>());
        wav.sample_rate = try!(reader.read_u32::<LittleEndian>());
        wav.byte_rate = try!(reader.read_u32::<LittleEndian>());
        wav.block_align = try!(reader.read_u16::<LittleEndian>());
        wav.bits_per_sample = try!(reader.read_u16::<LittleEndian>());
        Ok(())
    }

    fn parse_data_chunk_data(buf: &[u8], wav: &mut Wav) -> Result<(), WavLoadError> {
        let mut reader = Cursor::new(buf);
        let data_enum = match wav.bits_per_sample {
            8 => {
                let mut data = Vec::new();
                while let Ok(val) = reader.read_u8() {
                    data.push(val);
                }
                WavData::U8(data)
            },
            16 => {
                let mut data = Vec::new();
                while let Ok(val) = reader.read_i16::<LittleEndian>() {
                    data.push(val);
                }
                WavData::I16(data)
            },
            32 => {
                let mut data = Vec::new();
                while let Ok(val) = reader.read_i32::<LittleEndian>() {
                    data.push(val);
                }
                WavData::I32(data)
            },
            _ => return Err(WavLoadError::Parse(format!("Unexpected bits per sample value (got: \"{}\", expected 8, 16, or 32)", wav.bits_per_sample))),
        };

        wav.data = data_enum;
        Ok(())
    }

    fn buf_to_wav(buf: &[u8]) -> Result<Wav, WavLoadError> {
        let mut wav = Wav::new();

        let riff_chunk = Wav::parse_chunk(buf).unwrap();
        wav_assert!(&riff_chunk.id == "RIFF", format!("Unsupported wave file (got: \"{}\", expected \"RIFF\")", &riff_chunk.id));
        wav_assert!(riff_chunk.size as usize == buf.len() - 8, format!("Reported file size does not match actual size, {} vs {}", riff_chunk.size, buf.len()));

        let file_type = str::from_utf8(&buf[CHUNK_DATA_OFFSET..CHUNK_DATA_OFFSET + 4]).unwrap();
        wav_assert!(file_type == "WAVE", format!("Unsupported format (got: \"{}\", expected \"WAVE\")", file_type));

        let mut offset = CHUNK_DATA_OFFSET + 4;
        while let Some(chunk) = Wav::parse_chunk(&buf[offset..])
        {
            offset += CHUNK_DATA_OFFSET;
            match &chunk.id[..] {
                "fmt " => { Wav::parse_fmt_chunk_data(&buf[offset..], &mut wav); },
                "LIST" => (), //TODO implement
                "data" => { Wav::parse_data_chunk_data(&buf[offset..], &mut wav); },
                _ => return Err(WavLoadError::Parse(format!("Unable to parse chunk with id: {}", chunk.id).to_string())),
            }
            offset += chunk.size as usize;
        }

        Ok(wav)
    }
}
