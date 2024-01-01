//! Types for representing raw sounds inside the banks.

use std::io::Write;

use super::structures::SoundEntry;

use crate::error::*;

/// Represents the different sound formats supported.
///
/// The default is `Raw`, which is the proprietary format used in the original game.
///
/// `PcWav` is supported on PC builds for WAV audio.
///
/// `Ps2Vag` is supported on PlayStation 2 builds for VAG audio.
///
/// `Ps2Wav` is supported on PlayStation 2 builds if both `ps2` and `wav` features are enabled,
/// for WAV audio.
#[derive(Debug, Default, PartialEq, Eq)]
pub enum SoundType {
    #[default]
    Raw,
    #[cfg(feature = "pc")]
    PcWav,
    #[cfg(feature = "ps2")]
    Ps2Vag,
    #[cfg(all(feature = "ps2", feature = "wav"))]
    Ps2Wav,
}

impl SoundType {
    /// get the extension of the type.
    pub(crate) fn extension(&self) -> &'static str {
        match self {
            SoundType::Raw => "raw",
            #[cfg(feature = "pc")]
            SoundType::PcWav => "wav",
            #[cfg(feature = "ps2")]
            SoundType::Ps2Vag => "vag",
            #[cfg(all(feature = "ps2", feature = "wav"))]
            SoundType::Ps2Wav => "wav",
        }
    }
}

/// RawSounds is an iterator over the raw sound data contained in the
/// sound bank. It iterates over the sound entries, extracting the raw
/// sound data using the entry offset and size.
pub struct RawSounds<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) entries: &'a [SoundEntry],
    pub(crate) index: usize,
}

/// RawSound represents a raw sound extracted from the sound bank.
/// It the sample rate of the sound, and a slice referencing the raw sound bytes.
pub struct RawSound<'a> {
    pub(crate) index: usize,
    pub sample_rate: u16,
    pub bytes: &'a [u8],
}

impl<'a> RawSound<'a> {
    /// Write the raw sound to the writer.
    pub fn to_writer(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(self.bytes)?;
        Ok(())
    }
}

impl<'a> Iterator for RawSounds<'a> {
    type Item = RawSound<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let SoundEntry {
            offset,
            sample_rate,
            size,
            ..
        } = self.entries.get(self.index)?;

        // we don't check bounds at all, as we expect to have a valid input
        // at this point.
        let offset_start = *offset as usize;
        let offset_end = offset_start + *size;

        let raw_sound = RawSound {
            index: self.index,
            sample_rate: *sample_rate,
            bytes: &self.bytes[offset_start..offset_end],
        };

        self.index += 1;

        Some(raw_sound)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.entries.len() - self.index;
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for RawSounds<'a> {}
