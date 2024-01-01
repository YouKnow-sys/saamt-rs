use anyhow::bail;
use clap::{Subcommand, ValueEnum};

use crate::reporter::CliReporter;

mod config;
mod sfx;
mod stream;
mod utilities;

mod utils;

/// All program commands
#[derive(Debug, Subcommand)]
#[command(arg_required_else_help = true)]
pub enum Commands {
    /// Config related functions
    Config(config::ConfigCommands),
    /// Sfx archives related functions
    Sfx(sfx::SfxCommands),
    /// Stream archives related functions
    Stream,
    /// Other useful utilities
    #[command(subcommand, alias = "utility")]
    Utilities(utilities::UtilitiesCommands),
}

impl Commands {
    pub fn command(self, reporter: CliReporter) -> anyhow::Result<()> {
        match self {
            Self::Config(c) => c.command(reporter),
            Self::Sfx(c) => c.command(reporter),
            Self::Stream => bail!("Not yet implmented"),
            Self::Utilities(c) => c.command(reporter),
        }
    }
}

/// Different program log levels.
#[derive(Clone, Copy, Debug, Default, ValueEnum, PartialEq, Eq)]
pub enum LogLevel {
    /// Show all log messages and progress
    #[default]
    All,
    /// Show all log messages but no progress
    NoProgress,
    /// Only show Error and Warn messages
    Warn,
    /// Only show Error messages
    Error,
    /// Show nothing
    Nothing,
}
