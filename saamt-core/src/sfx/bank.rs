use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, Write},
    path::Path,
};

use binrw::{BinRead, BinWrite};

use crate::{
    error::*,
    config::lookuptable::LookUpEntry,
    reporter::{ProgressReport, ProgressReporterIterator},
};

use super::{
    sound::{RawSounds, SoundType},
    structures::BankHeader,
};

/// `Banks` struct loads banks from an SFX archive lazily.
pub struct Banks {
    lookup: Vec<(usize, LookUpEntry)>,
    lookup_idx: usize,
    reader: BufReader<File>,
}

impl Banks {
    pub(crate) fn new(reader: BufReader<File>, lookup: Vec<(usize, LookUpEntry)>) -> Self {
        Self {
            lookup,
            lookup_idx: 0,
            reader,
        }
    }

    /// Returns an iterator over the banks in this Banks instance.
    ///
    /// This allows lazily iterating over and processing the banks without
    /// loading them all into memory at once.
    pub fn banks_iter(self) -> BanksIter {
        BanksIter {
            lookup: self.lookup,
            lookup_idx: self.lookup_idx,
            reader: self.reader,
        }
    }

    /// Returns the number of banks in this Banks instance.
    pub fn len(&self) -> usize {
        self.lookup.len()
    }

    /// Checks if there are no banks in the reader for this Banks instance.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Exports all banks from the SFX archive to the given output directory.
    ///
    /// Iterates over each bank, exporting it to a .bnk file in the output
    /// directory named `bank_XXX.bnk` where `XXX` is the index of the bank.
    ///
    /// Reports progress of the export using the given progress reporter.
    ///
    /// Returns a Result with any errors encountered.
    pub fn export_all_banks(
        self,
        output_dir: impl AsRef<Path>,
        reporter: &mut impl ProgressReport,
    ) -> Result<()> {
        let output_dir = output_dir.as_ref();

        if !output_dir.is_dir() {
            std::fs::create_dir_all(output_dir)?;
        }

        let banks = self.banks_iter();

        let len = banks.len();
        for bank in banks.progress_report(reporter, len, "Saving banks".to_owned()) {
            let bank = bank?;
            let mut writer = BufWriter::new(File::create(
                output_dir.join(format!("bank_{:03}.bnk", bank.index)),
            )?);
            bank.to_writer(&mut writer)?;
            writer.flush()?;
        }

        Ok(())
    }

    /// Exports all sounds to the given [`SoundType`] format from all banks to the output directory.
    ///
    /// Iterates through each bank, extracting the sounds and saving them to the output directory.
    /// The sounds are organized into subdirs for each bank, named `bank_XXX` where `XXX` is the
    /// bank index.
    ///
    /// Sounds are named `sound_YYY.ext` where `YYY` is the sound index and `.ext` is the extension
    /// for the given sound type.
    ///
    /// Reports progress using the given progress reporter.
    pub fn export_all_sounds(
        self,
        sound_type: SoundType,
        output_dir: impl AsRef<Path>,
        reporter: &mut impl ProgressReport,
    ) -> Result<()> {
        let output_dir = output_dir.as_ref();

        let banks_len = self.len();
        for (bank, index) in self.banks_iter().zip(1..) {
            let bank = bank?;

            let output_dir = output_dir.join(format!("bank_{:03}", bank.index));
            if !output_dir.is_dir() {
                std::fs::create_dir_all(&output_dir)?;
            }

            for raw_sound in bank.raw_sounds().progress_report(
                reporter,
                bank.header.sound_entries.len(),
                format!("Bank ({index:03}/{banks_len:03})"),
            ) {
                let mut writer = BufWriter::new(File::create(output_dir.join(format!(
                    "sound_{:03}.{}",
                    raw_sound.index,
                    sound_type.extension()
                )))?);

                match sound_type {
                    SoundType::Raw => raw_sound.to_writer(&mut writer),
                    #[cfg(feature = "pc")]
                    SoundType::PcWav => raw_sound.as_pc_wav().to_writer(&mut writer),
                    #[cfg(feature = "ps2")]
                    SoundType::Ps2Vag => raw_sound.as_ps2_vag().to_writer(&mut writer),
                    #[cfg(all(feature = "ps2", feature = "wav"))]
                    SoundType::Ps2Wav => raw_sound.as_ps2_wav().to_writer(&mut writer),
                }?;

                writer.flush()?;
            }
        }

        Ok(())
    }
}

/// BanksIter is an iterator that lazily iterates over the banks in an SFX
/// archive.
///
/// This allows iterating over banks without having to load the entire SFX
/// file into memory. The banks are read on demand as the iterator is
/// advanced.
pub struct BanksIter {
    lookup: Vec<(usize, LookUpEntry)>,
    lookup_idx: usize,
    reader: BufReader<File>,
}

impl Iterator for BanksIter {
    type Item = Result<Bank>;

    fn next(&mut self) -> Option<Self::Item> {
        let (index, entry) = self.lookup.get(self.lookup_idx)?;
        self.lookup_idx += 1;

        Some(match BankHeader::read_args(&mut self.reader, entry.length as usize) {
            Ok(header) => {
                let mut bytes = vec![0_u8; entry.length as usize];

                if let Err(e) = self.reader.read_exact(&mut bytes) {
                    return Some(Err(Error::Io(e)));
                }

                Ok(Bank {
                    index: *index,
                    header,
                    bytes,
                })
            }
            Err(e) => Err(Error::BinRw(e)),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.lookup.len() - self.lookup_idx;
        (len, Some(len))
    }
}

impl ExactSizeIterator for BanksIter {}

/// Represents a sound effects (SFX) bank. Contains the bank header,
/// raw sound data bytes, and index of the bank.
pub struct Bank {
    /// index of bank inside the lookup index
    pub index: usize,
    pub header: BankHeader,
    pub bytes: Vec<u8>,
}

impl Bank {
    /// Provides access to the raw sounds inside this bank.
    pub fn raw_sounds(&self) -> RawSounds {
        RawSounds {
            bytes: &self.bytes,
            entries: &self.header.sound_entries,
            index: 0,
        }
    }

    /// Write the bank to the writer.
    pub fn to_writer<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        self.header.write(writer)?;
        writer.write_all(&self.bytes)?;
        Ok(())
    }

    /// Returns the total length of the bank in bytes, including the header size.
    pub fn len(&self) -> usize {
        self.bytes.len() + BankHeader::SIZE
    }

    /// is the bank have any bytes in it.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
