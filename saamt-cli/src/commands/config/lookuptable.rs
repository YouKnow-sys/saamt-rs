use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use clap::Subcommand;

use saamt_core::{config::lookuptable::LookUpTable, reporter::Logger};

use crate::reporter::CliReporter;

#[derive(Clone, Debug, Subcommand)]
pub enum LookupTableCommands {
    /// Export the lookup table
    Export,
    /// Create a lookup table
    Create,
}

impl LookupTableCommands {
    fn extension(&self) -> &'static str {
        match self {
            Self::Export => "json",
            Self::Create => "new.dat",
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

        reporter.info(format!("LookupTable action: {name}"));

        reporter.info("Opening input file.");
        let mut reader = BufReader::new(File::open(input)?);
        let mut writer = BufWriter::new(File::create(output)?);
        reporter.good("Input file opened.");

        match self {
            Self::Export => {
                let lookup_table = LookUpTable::from_reader(&mut reader)?;
                serde_json::to_writer_pretty(&mut writer, &lookup_table)?;
            }
            Self::Create => {
                let lookup_table: LookUpTable = serde_json::from_reader(&mut reader)?;
                lookup_table.to_writer(&mut writer)?;
            }
        }

        reporter.good(format!("{name} LookupTable finished."));

        Ok(())
    }
}
