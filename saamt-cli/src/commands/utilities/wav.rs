use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueHint};
use saamt_core::{reporter::Logger, utils::wav::Wav};

use crate::{commands::utils, reporter::CliReporter};

#[derive(Debug, Parser)]
pub struct WavCommands {
    /// Input file
    #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
    input: PathBuf,
    /// Wav action
    #[command(subcommand)]
    action: Action,
}

#[derive(Clone, Debug, Subcommand)]
enum Action {
    /// Dump Wav spec
    Dump,
}

impl WavCommands {
    pub fn command(self, mut reporter: CliReporter) -> anyhow::Result<()> {
        reporter.info(format!("Wav utility action: {:?}", self.action));

        match self.action {
            Action::Dump => {
                reporter.info("Opening Wav file.");
                let wav = Wav::from_file(self.input)?;
                reporter.good("Wav file loaded.");

                reporter.good(format!("Wav spec:\n{:#?}", wav.spec()));
            }
        }

        Ok(())
    }
}
