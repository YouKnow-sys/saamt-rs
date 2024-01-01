//! SFX archive manager.

use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::{BufReader, BufWriter, Cursor, Write},
    path::{Path, PathBuf},
};

use crate::{
    error::*,
    config::lookuptable::{LookUpEntry, LookUpTable},
    config::paknames::PakNames,
    reporter::{Logger, ProgressReport, ProgressReporterIterator},
    utils,
};

use bank::Banks;

use self::{sound::SoundType, structures::BankHeader};

pub mod bank;
#[cfg(any(feature = "ps2", feature = "pc"))]
mod platforms;
pub mod sound;
mod structures;

type SortedLookupReturn = (Vec<(usize, LookUpEntry)>, Vec<usize>, bool);

/// ## SFXManager
/// SFXManager manages loading and modifying SFX archives. It contains
/// the lookup table and PAK names needed to process SFX files.
#[derive(Clone, Debug)]
pub struct SfxManager {
    lookup_path: PathBuf,
    pub lookup_table: LookUpTable,
    pak_names: PakNames,
}

impl SfxManager {
    /// Creates a new `SfxManager` instance by loading the lookup table from the provided `lookup_file` path
    /// and the pak names from the optional `pakfile_dat_file`.
    ///
    /// The `lookup_file` path is saved and used later when updating the lookup table.
    ///
    /// Logging output is written to the provided `logger`.
    ///
    /// Returns a `Result` with the `SfxManager` instance or a error if loading fails.
    pub fn new<P, L>(lookup_file: P, pakfile_dat_file: Option<P>, logger: &mut L) -> Result<Self>
    where
        P: AsRef<Path>,
        L: Logger,
    {
        let lookup_file = lookup_file.as_ref();

        logger.info("Loading lookup table.");
        let lookup_table = {
            let mut reader = BufReader::new(File::open(lookup_file)?);
            LookUpTable::from_reader(&mut reader)?
        };
        logger.good("Lookup table loaded.");

        logger.info("Loading Pak names.");
        let pak_names = match pakfile_dat_file {
            Some(pdf /* :D */) => {
                let mut reader = BufReader::new(File::open(pdf)?);
                PakNames::sfx_from_reader(&mut reader)?
            }
            None => PakNames::sfx(), // use default sfx names
        };
        logger.good("Pak names loaded.");

        Ok(Self {
            lookup_path: lookup_file.to_path_buf(),
            lookup_table,
            pak_names,
        })
    }

    /// Load a sfx archive and return a [`SfxArchive`].
    pub fn load(&self, sfx_pak: impl AsRef<Path>, logger: &mut impl Logger) -> Result<SfxArchive> {
        let sfx_pak = sfx_pak.as_ref();

        logger.info("Getting Banks entry based on SFX archive name.");
        let (lookup, indexes, sorted) = self.get_sorted_lookup_table(sfx_pak)?;
        if sorted {
            logger.warn("Lookup entries were not sorted, it should be ok but as I didn't test any sfx archive that isn't sorted it may cause some problems.");
        }
        logger.info("Banks entries generated.");

        logger.info("Opening SFX archive.");
        let reader = BufReader::new(File::open(sfx_pak)?);
        logger.good("SFX archive opened.");

        Ok(SfxArchive::new(reader, lookup, indexes))
    }

    /// Update and save the lookup table.
    ///
    /// `path` is optional, if `path` is `None` the original Lookup
    /// file will be updated
    ///
    /// # Note:
    /// please note that you need to call this function after loading and creating/updating new sfx files
    /// using [`SfxArchive`].
    /// if you don't call this method the lookup file wont get updated and game wont work.
    pub fn update_lookup(&self, path: Option<PathBuf>) -> Result<()> {
        let path = path.unwrap_or(self.lookup_path.clone());
        let mut writer = BufWriter::new(File::create(path)?);
        self.lookup_table.to_writer(&mut writer)?;
        writer.flush()?;

        Ok(())
    }

    /// Try to get the sorted lookup table based on the input path basename.
    // Im almost sure there is no need for do all this, but I'll do it anyway...
    fn get_sorted_lookup_table(&self, path: &Path) -> Result<SortedLookupReturn> {
        /// Check if banks inside the lookup are sorted based on offset.
        fn is_banks_sorted(lookup: &[(usize, (usize, LookUpEntry))]) -> bool {
            lookup.windows(2).all(|e| {
                (e[0].1 .1.offset + e[0].1 .1.length) as usize + structures::BankHeader::SIZE
                    == e[1].1 .1.offset as usize
            })
        }

        let basename = path.with_extension("");
        let Some(basename) = basename.file_name().and_then(OsStr::to_str) else {
            return Err(Error::CantGetBaseName(format!("{}", path.display())));
        };

        // Determine lookup index which is necessary for determining some of
        // the sound lengths and will be put in the INI file to help importing.
        // We will conveniently use the ALL CAPS basename for this.
        let Some(lookup_idx) = self.pak_names.get_pak_idx_from_name(basename) else {
            return Err(Error::CantFindInLookupTable);
        };

        // The index is valid, but are there entries for it in the lookup file?
        let num_banks = self.lookup_table.count_entries_matching_pak_idx(lookup_idx);
        if num_banks == 0 {
            return Err(Error::NoEntryMatch);
        }

        let mut lookup: Vec<_> = self
            .lookup_table
            .matching_entries(lookup_idx)
            .into_iter()
            .enumerate()
            .map(|(i2, (i1, e))| (i1, (i2, e)))
            .collect();

        let mut sorted = false;
        // check if the banks are sorted or not
        if !is_banks_sorted(&lookup) {
            // sort it, it seem unnecessary to me because entries are already back to back
            // but its always good to be on the safe side
            lookup.sort_by(|(_, (_, e1)), (_, (_, e2))| e1.offset.cmp(&e2.offset));
            sorted = true;
            if !is_banks_sorted(&lookup) {
                // if the bank isn't still sorted we just return an error, this shouldn't ever happen
                return Err(Error::UnsortedSfxBanks);
            }
        }
        // at this point we are sure that banks are sorted!

        let (indexes, lookup): (Vec<_>, Vec<_>) = lookup.into_iter().unzip();

        Ok((lookup, indexes, sorted))
    }
}

/// Loaded sfx archive that have the banks inside it.
pub struct SfxArchive {
    /// Banks inside the sfx archive.
    banks: Banks,
    /// Original indexes of banks inside lookup table.
    indexes: Vec<usize>,
}

impl SfxArchive {
    fn new(
        reader: BufReader<File>,
        lookup: Vec<(usize, LookUpEntry)>,
        indexes: Vec<usize>,
    ) -> Self {
        Self {
            banks: Banks::new(reader, lookup),
            indexes,
        }
    }

    /// get the banks inside the archive.
    pub fn banks(self) -> Banks {
        self.banks
    }

    /// Imports previously exported .bnk files back into a new sfx archive.
    ///
    /// # Note:
    /// keep in mind that the input folder that you used to load banks in first place
    /// shouldn't be the same as the `output_path`.
    pub fn import_banks(
        self,
        input_path: impl AsRef<Path>,
        output: impl AsRef<Path>,
        lookuptbl: &mut LookUpTable,
        reporter: &mut (impl ProgressReport + Logger),
    ) -> Result<()> {
        reporter.info("Generating file list.");
        let files = utils::generate_file_list(input_path, Some(&["bnk"]), 1);
        reporter.good("File list generated.");

        if files.is_empty() {
            return Err(Error::NoFileFound("bnk"));
        }

        let files: HashMap<_, _> = files
            .into_iter()
            .filter_map(|f| {
                let fe = f.with_extension("");
                let (name, num) = fe
                    .file_name()
                    .and_then(OsStr::to_str)
                    .and_then(|n| n.split_once('_'))?;

                if name != "bank" {
                    return None;
                }

                num.parse::<usize>().map(|n| (n, f)).ok()
            })
            .collect();

        if files.is_empty() {
            return Err(Error::NoFileFound("valid bnk"));
        }

        reporter.good(format!("Found {} bank.", files.len()));

        let mut writer = BufWriter::with_capacity(1024 * 1024, File::create(output)?);
        let mut offset = 0;

        let len = self.banks.len();
        for (bank, index) in self.banks.banks_iter().zip(self.indexes).progress_report(
            reporter,
            len,
            "Importing banks".to_owned(),
        ) {
            let bank = bank?;
            let Some(entry) = lookuptbl.get_mut(index) else {
                return Err(Error::CantFindIndexInLookUpTable);
            };

            entry.offset = offset;

            match files.get(&bank.index) {
                Some(path) => {
                    let buf = std::fs::read(path)?;
                    offset += buf.len() as u32;
                    entry.length = (buf.len() - BankHeader::SIZE) as u32;

                    writer.write_all(&buf)?;
                }
                None => {
                    offset += bank.len() as u32;
                    entry.length = bank.bytes.len() as u32;

                    bank.to_writer(&mut writer)?;
                }
            }
        }

        writer.flush()?;

        reporter.good("Import finished and a new archive created.");

        Ok(())
    }

    /// Import sound data back to banks and then create a new sfx archive from the banks.
    ///
    /// You need to choose what kind of sound you exported previously, so program only import those types.
    pub fn import_sounds(
        self,
        sound_type: SoundType,
        input_path: impl AsRef<Path>,
        output: impl AsRef<Path>,
        lookuptbl: &mut LookUpTable,
        reporter: &mut (impl ProgressReport + Logger),
    ) -> Result<()> {
        let input_path = input_path.as_ref();

        reporter.info("Generating folder list.");
        let folders = utils::generate_folder_list(input_path, 1);
        reporter.good("Folder list generated.");

        if folders.is_empty() {
            return Err(Error::NoFolderFound("bank"));
        }

        reporter.info("Generating file list.");
        let folders: HashMap<usize, Vec<usize>> = folders
            .into_iter()
            .filter_map(|f| {
                let (name, num) = f
                    .file_name()
                    .and_then(OsStr::to_str)
                    .and_then(|n| n.split_once('_'))?;

                if name != "bank" {
                    return None;
                }

                let bank_num = num.parse::<usize>().ok()?;

                let files = utils::generate_file_list(f, Some(&[sound_type.extension()]), 1);

                (!files.is_empty()).then(|| {
                    (
                        bank_num,
                        files
                            .into_iter()
                            .filter_map(|f| {
                                let fe = f.with_extension("");
                                let (name, num) = fe
                                    .file_name()
                                    .and_then(OsStr::to_str)
                                    .and_then(|n| n.split_once('_'))?;

                                if name != "sound" {
                                    return None;
                                }

                                num.parse::<usize>().ok()
                            })
                            .collect::<Vec<_>>(),
                    )
                })
            })
            .filter(|(_, files)| !files.is_empty())
            .collect();

        reporter.good("File list generated.");

        if folders.is_empty() {
            return Err(Error::NoFileFound(sound_type.extension()));
        }

        reporter.good(format!(
            "Found {} folders with valid {} files in them.",
            folders.len(),
            sound_type.extension()
        ));

        let mut output_writer = BufWriter::with_capacity(1024 * 1024, File::create(output)?);

        let mut offset = 0;
        let mut not_mono = false;

        let len = self.banks.len();
        for (bank, index) in self.banks.banks_iter().zip(self.indexes).progress_report(
            reporter,
            len,
            "Importing Sound/banks".to_owned(),
        ) {
            let mut bank = bank?;
            let Some(entry) = lookuptbl.get_mut(index) else {
                return Err(Error::CantFindIndexInLookUpTable);
            };

            entry.offset = offset;

            if let Some(files) = folders.get(&bank.index) {
                let mut soffset = 0;
                let mut bytes_writer = Cursor::new(Vec::with_capacity(bank.bytes.len()));

                for (index, sentry) in bank.header.sound_entries.iter_mut().enumerate() {
                    sentry.offset = soffset;

                    if files.contains(&index) {
                        let path = input_path.join(format!(
                            "bank_{:03}/sound_{index:03}.{}",
                            bank.index,
                            sound_type.extension()
                        ));

                        not_mono = match sound_type {
                            SoundType::Raw => {
                                platforms::raw::import_raw(&path, sentry, &mut bytes_writer)
                            }
                            #[cfg(feature = "pc")]
                            SoundType::PcWav => {
                                platforms::pc::import_wav(&path, sentry, &mut bytes_writer)
                            }
                            #[cfg(feature = "ps2")]
                            SoundType::Ps2Vag => {
                                platforms::ps2::import_vag(&path, sentry, &mut bytes_writer)
                            }
                            #[cfg(all(feature = "ps2", feature = "wav"))]
                            SoundType::Ps2Wav => {
                                platforms::ps2::import_wav(&path, sentry, &mut bytes_writer)
                            }
                        }?;
                    } else {
                        let offset_start = sentry.offset as usize;
                        let offset_end = offset_start + sentry.size;

                        bytes_writer.write_all(&bank.bytes[offset_start..offset_end])?;
                    }

                    soffset += sentry.size as u32;
                }

                bank.bytes = bytes_writer.into_inner();
            }

            entry.length = bank.bytes.len() as u32;
            offset += bank.len() as u32;

            bank.to_writer(&mut output_writer)?;
        }

        if not_mono {
            reporter.warn("One or more of wav files wasn't mono, game may have problem in reading sfx files that have none mono audio in them.");
        }

        output_writer.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    // TODO: this module need more tests, but atm I really don't want to...

    use crate::reporter::{Logger, ProgressReport};

    use super::*;

    struct TestLogger;

    impl Logger for TestLogger {
        fn info(&mut self, _: impl AsRef<str>) {}

        fn good(&mut self, _: impl AsRef<str>) {}

        fn warn(&mut self, str: impl AsRef<str>) {
            panic!("SFX sent a warn msg: {}", str.as_ref());
        }

        fn error(&mut self, str: impl AsRef<str>) {
            panic!("SFX sent a error msg: {}", str.as_ref());
        }
    }

    impl ProgressReport for TestLogger {
        fn begin_progress(&mut self, _: String, _: usize) {}

        fn add_progress(&mut self) {}

        fn end_progress(&mut self) {}
    }

    #[test]
    fn pc() {
        let mut logger = TestLogger;

        let sfx_manager = SfxManager::new("test-assets/PC/BankLkup.dat", None, &mut logger)
            .expect("failed to open archive");

        let archive = sfx_manager
            .load("test-assets/PC/FEET", &mut logger)
            .expect("failed to load archive");

        // check the len
        assert_eq!(archive.banks.len(), 7);

        for bank in archive.banks().banks_iter() {
            bank.expect("Can't read the bank");
        }
    }

    #[test]
    fn ps2() {
        let mut logger = TestLogger;

        let sfx_manager = SfxManager::new("test-assets/PS2/BankLkup.dat", None, &mut logger)
            .expect("failed to open archive");

        let archive = sfx_manager
            .load("test-assets/PS2/FEET01.pak", &mut logger)
            .expect("failed to load archive");

        // check the len
        assert_eq!(archive.banks.len(), 7);

        for bank in archive.banks().banks_iter() {
            bank.expect("Can't read the bank");
        }
    }
}
