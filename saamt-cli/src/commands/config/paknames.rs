use std::{
    ffi::OsStr,
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

use clap::Subcommand;

use saamt_core::{config::paknames::PakNames, reporter::Logger};

use crate::reporter::CliReporter;

#[derive(Clone, Debug, Subcommand)]
pub enum PakNamesCommands {
    /// Dump the PakNames to json
    #[cfg(feature = "serde")]
    Dump,
    /// Export the pak names
    Export,
}

impl PakNamesCommands {
    fn extension(&self) -> &'static str {
        match self {
            Self::Dump => "json",
            Self::Export => "txt",
        }
    }

    pub fn command(
        self,
        input: PathBuf,
        output: Option<PathBuf>,
        mut reporter: CliReporter,
    ) -> anyhow::Result<()> {
        let output = output.unwrap_or_else(|| input.with_extension(self.extension()));

        let name = format!("{:?}", self);

        reporter.info(format!("PakNames action: {name}"));

        let pak_name = input
            .with_extension("")
            .file_name()
            .and_then(OsStr::to_str)
            .map(ToOwned::to_owned);
        anyhow::ensure!(pak_name.is_some(), "Failed to get input pak name");

        reporter.info("Opening and loading PakNames file.");
        let mut reader = BufReader::new(File::open(input)?);
        let pak_names = PakNames::from_reader(&pak_name.unwrap(), &mut reader)?;
        let mut writer = BufWriter::new(File::create(output)?);
        reporter.good("PakName file loaded.");

        match self {
            #[cfg(feature = "serde")]
            Self::Dump => serde_json::to_writer_pretty(&mut writer, &pak_names)?,
            Self::Export => {
                for s in pak_names.iter() {
                    writeln!(&mut writer, "{s}")?;
                }
            }
        }

        reporter.good(format!("{name} PakName finished."));

        Ok(())
    }
}
