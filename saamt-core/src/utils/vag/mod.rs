//! A set of function for creating, encoding, decoding and managing sony ps2 vag files.

use std::{
    fmt::Debug,
    fs::File,
    io::{BufWriter, Cursor, Seek, Write},
    mem::size_of,
    path::Path,
};

use binrw::{binrw, io::BufReader, BinRead, BinWrite};

use crate::error::*;

use decoder::VAG2WAVDecoder;
#[cfg(feature = "wav")]
use encoder::{LoopMode, WAV2VAGEncoder};

#[cfg(feature = "wav")]
use super::wav::Wav;

pub mod decoder;
#[cfg(feature = "wav")]
pub mod encoder;

/// The number of samples in each VAG chunk
const VAG_SAMPLE_BYTES: usize = 14;
const VAG_SAMPLE_NIBBL: usize = VAG_SAMPLE_BYTES * 2;

/// A wrapper for the underlying Vag audio
pub struct VagAudio(pub(crate) Vag);

impl From<Vag> for VagAudio {
    fn from(value: Vag) -> Self {
        Self(value)
    }
}

impl VagAudio {
    /// Create a new Vag file from input wav file.
    #[cfg(feature = "wav")]
    pub fn from_wav(wav_path: impl AsRef<Path>, loop_mode: LoopMode) -> Result<Self> {
        WAV2VAGEncoder::new(wav_path.as_ref(), loop_mode).map(|w2v| w2v.generate_vag())
    }

    /// Read a vag file from file.
    pub fn from_file(vag_path: impl AsRef<Path>) -> Result<Self> {
        let mut reader = BufReader::new(File::open(vag_path)?);
        Ok(Vag::read(&mut reader)?.into())
    }

    /// Write vag audio file to disk.
    pub fn to_disk(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut writer = BufWriter::new(File::create(path)?);
        self.to_writer(&mut writer)?;
        writer.flush()?;

        Ok(())
    }

    /// Write the vag to the writer.
    pub fn to_writer<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        self.0.write(writer)?;
        Ok(())
    }

    /// Get the vag file name
    pub fn name(&self) -> String {
        self.0.name()
    }

    /// First convert vag to wav and then save the wav file to disk using MFAudio.
    #[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
    pub fn save_as_wav_mfaudio(&self, path: impl AsRef<Path>) -> Result<()> {
        let vag_path = path.as_ref().with_extension("vag");
        self.to_disk(&vag_path)?;

        super::mfaudio::convert(super::mfaudio::MFAudioType::Wavu, &vag_path, path.as_ref())
    }

    /// Decode and return vag as wav
    #[cfg(feature = "wav")]
    pub fn to_wav(&self) -> Wav {
        self.decoder().to_wav()
    }

    /// Get the vag bytes without vag header.\
    /// at this point we expect the vag to be valid,
    /// so we will panic in any kind of error.
    pub fn raw_vag_bytes(&self) -> Vec<u8> {
        let vag = &self.0;

        let mut writer = Cursor::new(Vec::with_capacity(
            (vag.chunks.len() * size_of::<VAGChunk>()) + 16,
        ));
        writer
            .write_all(&vag.vag_header)
            .expect("Failed to write vag header");

        for chunk in vag.chunks.iter() {
            chunk
                .write(&mut writer)
                .expect("Failed to write the vag chunk.");
        }

        writer.into_inner()
    }

    /// Create a decoder that decode Vag to wav
    pub fn decoder(&self) -> VAG2WAVDecoder {
        VAG2WAVDecoder::new(self)
    }
}

impl Debug for VagAudio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Vag audio file (PS2)
#[binrw]
#[brw(big, magic = b"VAGp")]
#[derive(Debug)]
pub(crate) struct Vag {
    pub version: u32,
    ssa: u32,
    #[br(temp, assert((size - 16) as usize % size_of::<VAGChunk>() == 0, "Invalid vag file, size doesn't match VAGChunk number"))]
    #[bw(calc = ((chunks.len() * size_of::<VAGChunk>()) + 16 /* vag header size */) as u32)]
    size: u32,
    pub sample_rate: u32,
    vol_left: i16,
    vol_right: i16,
    pitch: i16,
    adsr1: i16,
    adsr2: i16,
    #[brw(assert(channels.le(&1), "We currently only support single channel Vag files"))]
    channels: u16,
    name: [u8; 16],
    vag_header: [u8; 16],
    #[brw(little)]
    #[br(count = (size - 16) as usize / size_of::<VAGChunk>())]
    pub chunks: Vec<VAGChunk>,
}

impl Vag {
    /// Create a new vag file based on input.
    ///
    /// ## Warning
    /// keep in mind that we wont return any errors in this
    /// function, if user pass a invalid vag file to this method
    /// we just panic.
    pub fn new(sample_rate: u32, name: [u8; 16], data: Vec<u8>) -> Self {
        assert!(
            data.len() > 16,
            "The vag data need to at least be bigger then 16 byte."
        );

        let (vag_header, data) = data.split_at(16);

        assert!(
            data.len() % size_of::<VAGChunk>() == 0,
            "Invalid vag, data size doesn't fit a valid number of Chunks."
        );

        let chunks: Vec<VAGChunk> = data
            .chunks_exact(16)
            .map(|slice| VAGChunk {
                pack_infos: PackInfo(slice[0]),
                flags: VAGFlag::try_from(slice[1]).expect("Invalid flag value in VAG"),
                sample: slice[2..16]
                    .try_into()
                    .expect("Failed to read samples to VAGChunk."),
            })
            .collect();

        Self {
            version: 0x20,
            ssa: 0x0,
            sample_rate,
            vol_left: 0,
            vol_right: 0,
            pitch: 0,
            adsr1: 0,
            adsr2: 0,
            channels: 0,
            name,
            vag_header: vag_header.try_into().unwrap(),
            chunks,
        }
    }

    /// Create a new vag from the chunks
    #[cfg(feature = "wav")]
    pub fn new_from_chunks(sample_rate: u32, name: [u8; 16], chunks: Vec<VAGChunk>) -> Self {
        Self {
            version: 0x20,
            ssa: 0x0,
            sample_rate,
            vol_left: 0,
            vol_right: 0,
            pitch: 0,
            adsr1: 0,
            adsr2: 0,
            channels: 0,
            name,
            vag_header: Default::default(),
            chunks,
        }
    }

    /// Get the name of vag file, remember that this method
    /// convert to string lossy, this mean unknown character
    /// will get replaced.
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

#[binrw]
#[brw(repr(u8))]
#[derive(Debug, Default, PartialEq, Eq)]
enum VAGFlag {
    #[default]
    Nothing = 0, // Nothing
    LoopLastBlock = 1,  // Last block to loop
    LoopRegion = 2,     // Loop region
    LoopEnd = 3,        // Ending block of the loop
    LoopFirstBlock = 4, // First block of looped data
    Unk = 5,            // ?
    LoopStart = 6,      // Starting block of the loop
    PlaybackEnd = 7,    // Playback ending position
}

impl TryFrom<u8> for VAGFlag {
    type Error = u8;

    fn try_from(value: u8) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Nothing,
            1 => Self::LoopLastBlock,
            2 => Self::LoopRegion,
            3 => Self::LoopEnd,
            4 => Self::LoopFirstBlock,
            5 => Self::Unk,
            6 => Self::LoopStart,
            7 => Self::PlaybackEnd,
            n => return Err(n),
        })
    }
}

/// # VagChunk
/// each chunk hold information about the samples and the samples themselves.
#[binrw]
#[derive(Default)]
#[brw(little)]
pub struct VAGChunk {
    pack_infos: PackInfo,
    flags: VAGFlag,
    sample: [u8; VAG_SAMPLE_BYTES],
}

impl Debug for VAGChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VAGChunk")
            .field("pack_infos", &self.pack_infos)
            .field("flags", &self.flags)
            // .field("sample", &self.sample)
            .finish()
    }
}

#[binrw]
#[repr(transparent)]
#[derive(Clone, Copy, Default)]
pub struct PackInfo(u8);

impl PackInfo {
    pub fn shift_factor(&self) -> i8 {
        (self.0 & 0xf) as i8
    }

    pub fn predict(&self) -> i8 {
        (self.0 >> 4) as i8
    }

    pub fn update_shift_factor(&mut self, value: i8) {
        self.0 = (((self.predict() << 4) & 0xF0u8 as i8) | (value & 0x0F)) as u8;
    }

    pub fn update_predict(&mut self, value: i8) {
        self.0 = (((value << 4) & 0xF0u8 as i8) | (self.shift_factor() & 0x0F)) as u8;
    }
}

impl Debug for PackInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackInfo")
            .field("shift_factor", &self.shift_factor())
            .field("predict", &self.predict())
            .finish()
    }
}
