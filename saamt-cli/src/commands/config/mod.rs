use std::{ffi::OsStr, path::PathBuf};

use anyhow::bail;
use clap::{Parser, Subcommand, ValueHint};
use saamt_core::reporter::Logger;

use crate::reporter::CliReporter;

use self::paknames::PakNamesCommands;

use super::utils;

use bankslot::BankSlotCommands;
use lookuptable::LookupTableCommands;

mod bankslot;
#[cfg(feature = "serde")]
mod lookuptable;
mod paknames;

#[derive(Debug, Parser)]
pub struct ConfigCommands {
    /// Input dat config
    #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
    input_dat: PathBuf,
    /// Output file
    output: Option<PathBuf>,
    /// Type of config
    #[command(subcommand)]
    config_type: ConfigType,
}

#[derive(Clone, Debug, Default, Subcommand)]
enum ConfigType {
    /// Try to autodetect the Config type and dump it
    #[cfg(feature = "serde")]
    #[default]
    Auto,
    /// BankSlot related functions
    #[command(subcommand)]
    BankSlot(BankSlotCommands),
    /// LookupTable related functions (BankLkup, TrakLkup)
    #[cfg(feature = "serde")]
    #[command(subcommand)]
    LookupTable(LookupTableCommands),
    /// PakNames related functions (BankSlot, StrmPaks)
    #[command(subcommand)]
    PakNames(PakNamesCommands),
}

impl ConfigType {
    const fn name(&self) -> &str {
        match self {
            #[cfg(feature = "serde")]
            ConfigType::Auto => "Auto",
            ConfigType::BankSlot(_) => "BankSlot",
            ConfigType::LookupTable(_) => "LookupTable",
            ConfigType::PakNames(_) => "PakNames",
        }
    }
}

impl ConfigCommands {
    pub fn command(self, mut reporter: CliReporter) -> anyhow::Result<()> {
        reporter.info(format!("Config type: {}", self.config_type.name()));

        match self.config_type {
            #[cfg(feature = "serde")]
            ConfigType::Auto => {
                let Some(name) = self
                    .input_dat
                    .with_extension("")
                    .file_name()
                    .and_then(OsStr::to_str)
                    .map(str::to_lowercase)
                else {
                    bail!("Can't get the input filename.");
                };

                match name.as_ref() {
                    "bankslot" => {
                        BankSlotCommands::Dump.command(self.input_dat, self.output, reporter)
                    }
                    #[cfg(feature = "serde")]
                    "banklkup" | "traklkup" => {
                        LookupTableCommands::Export.command(self.input_dat, self.output, reporter)
                    }
                    "pakfiles" | "strmpaks" => {
                        PakNamesCommands::Dump.command(self.input_dat, self.output, reporter)
                    }
                    name => {
                        bail!("Can't detect the type of config based on the file name: {name}.")
                    }
                }
            }
            ConfigType::BankSlot(c) => c.command(self.input_dat, self.output, reporter),
            #[cfg(feature = "serde")]
            ConfigType::LookupTable(c) => c.command(self.input_dat, self.output, reporter),
            ConfigType::PakNames(c) => c.command(self.input_dat, self.output, reporter),
        }
    }
}
