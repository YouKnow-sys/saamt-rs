use clap::Subcommand;

use crate::reporter::CliReporter;

mod vag;
#[cfg(feature = "wav")]
mod wav;

#[derive(Debug, Subcommand)]
pub enum UtilitiesCommands {
    /// Vag related functions and utilities
    Vag(vag::VagCommands),
    /// Wav related functions and utilities
    #[cfg(feature = "wav")]
    Wav(wav::WavCommands),
}

impl UtilitiesCommands {
    pub fn command(self, reporter: CliReporter) -> anyhow::Result<()> {
        match self {
            UtilitiesCommands::Vag(c) => c.command(reporter),
            #[cfg(feature = "wav")]
            UtilitiesCommands::Wav(c) => c.command(reporter),
        }
    }
}
