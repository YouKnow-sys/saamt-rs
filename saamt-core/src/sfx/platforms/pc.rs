use std::{
    io::{Cursor, Seek, Write},
    mem::size_of,
    path::Path,
};

use binrw::BinWrite;
use hound::{SampleFormat, WavSpec};

use crate::{
    error::*,
    sfx::{
        bank::Bank,
        sound::{RawSound, RawSounds},
        structures::SoundEntry,
    },
    utils::{helpers::DataSaveAll, wav::Wav},
};

/// Imports a WAV file from the given path into the provided SoundEntry and bytes writer.
///
/// Loads the WAV file, copies the sample rate and size into the SoundEntry,
/// writes the WAV samples to the bytes writer in little endian format,
/// and returns whether the WAV had more than 1 channel.
pub fn import_wav(
    path: &Path,
    sentry: &mut SoundEntry,
    bytes_writer: &mut Cursor<Vec<u8>>,
) -> Result<bool> {
    let wav = Wav::from_file(path)?;

    sentry.sample_rate = wav.spec.sample_rate as _;
    sentry.size = wav.samples.len() * size_of::<i16>();

    wav.samples.write_le(bytes_writer)?;

    Ok(wav.spec.channels != 1)
}

/// Iterator over raw sounds converted to PC WAV format.
///
/// Wraps a `RawSounds` iterator and converts each raw sound to PC WAV
/// when iterating. This allows iterating over sounds in PC WAV format
/// without having to do the conversion upfront.
pub struct PCSounds<'a>(RawSounds<'a>);

impl<'a> Iterator for PCSounds<'a> {
    type Item = Wav;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|rs| rs.as_pc_wav())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> ExactSizeIterator for PCSounds<'a> {}

impl<'a> DataSaveAll for PCSounds<'a> {
    fn fullname(index: usize) -> String {
        format!("sound_{index:03}.wav")
    }

    fn write<W: Write + Seek>(data: Self::Item, writer: &mut W) -> Result<()> {
        data.to_writer(writer)
    }
}

impl<'a> From<RawSounds<'a>> for PCSounds<'a> {
    fn from(value: RawSounds<'a>) -> Self {
        PCSounds(value)
    }
}

impl Bank {
    /// Returns an iterator over the raw sounds from this bank
    /// converted to PC WAV format.
    ///
    /// This should only be used if you are certain that all sounds in the bank  
    /// originate from the PC version of the game. Otherwise, use
    /// `raw_sounds()` to get the raw sounds before converting.
    pub fn pc_sounds(&self) -> PCSounds {
        self.raw_sounds().into()
    }
}

impl<'a> RawSound<'a> {
    /// Converts the raw sound samples into a WAV format for PC.
    ///
    /// Creates a `Wav` struct with the sound converted to 16-bit PCM samples
    /// at the source sample rate and mono channel. No validation of the raw
    /// samples is performed.
    pub fn as_pc_wav(&self) -> Wav {
        let spec = WavSpec {
            channels: 1,
            sample_rate: self.sample_rate as _,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let samples: Vec<i16> = self
            .bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        Wav { samples, spec }
    }
}
