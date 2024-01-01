use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
};

use clap::{Subcommand, ValueHint};

use saamt_core::{config::bankslot::BankSlot, reporter::Logger};

use crate::{commands::utils, reporter::CliReporter};

#[derive(Clone, Debug, Subcommand)]
pub enum BankSlotCommands {
    /// Dump the BankSlot content to json
    #[cfg(feature = "serde")]
    Dump,
    /// Export the buffer sizes
    Export,
    /// Import the buffer sizes
    #[command(arg_required_else_help = true)]
    Import {
        /// Txt file that have the buffer sizes
        #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
        txt: PathBuf,
    },
}

impl BankSlotCommands {
    const fn name(&self) -> &str {
        match self {
            BankSlotCommands::Dump => "Dump",
            BankSlotCommands::Export => "Export",
            BankSlotCommands::Import { .. } => "Import",
        }
    }
}

impl BankSlotCommands {
    fn extension(&self) -> &'static str {
        match self {
            #[cfg(feature = "serde")]
            Self::Dump => "json",
            Self::Export => "txt",
            Self::Import { .. } => "new.dat",
        }
    }

    pub fn command(
        self,
        input: PathBuf,
        output: Option<PathBuf>,
        mut reporter: CliReporter,
    ) -> anyhow::Result<()> {
        let output = output.unwrap_or_else(|| input.with_extension(self.extension()));

        let name = self.name().to_owned();

        reporter.info(format!("BankSlot action: {name}"));

        reporter.info("Opening and loading BankSlot.");
        let mut reader = BufReader::new(File::open(input)?);
        let mut bank_slot = BankSlot::from_reader(&mut reader)?;
        let mut writer = BufWriter::new(File::create(output)?);
        reporter.good("BankSlot loaded.");

        match self {
            #[cfg(feature = "serde")]
            Self::Dump => serde_json::to_writer_pretty(&mut writer, &bank_slot)?,
            Self::Export => {
                for s in bank_slot.export_buf_sizes() {
                    writeln!(&mut writer, "{s}")?;
                }
            }
            Self::Import { txt } => {
                let sizes: Vec<u32> = BufReader::new(File::open(txt)?)
                    .lines()
                    .map_while(Result::ok)
                    .filter_map(|l| l.parse::<u32>().ok())
                    .collect();

                anyhow::ensure!(
                    sizes.len() == bank_slot.export_buf_sizes().len(),
                    "the bank slots txt len isn't the same as original"
                );

                bank_slot.update_buf_sizes(sizes);

                bank_slot.to_writer(&mut writer)?;
            }
        }

        reporter.good(format!("BankSlot {name} finished."));

        Ok(())
    }
}
