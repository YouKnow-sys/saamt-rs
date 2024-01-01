use std::{ffi::OsStr, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum, ValueHint};

use saamt_core::{reporter::Logger, sfx_prelude::*};

use crate::{commands::utils, reporter::CliReporter};

#[derive(Debug, Parser)]
#[command(arg_required_else_help = true)]
pub struct SfxCommands {
    /// What to do
    #[command(subcommand)]
    action: Action,
    /// Path to the input file
    #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
    input_archive: PathBuf,
    /// Path to lookup table file
    #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
    lookup_table: PathBuf,
    /// Optional path to pak names file (PakFiles.dat)
    #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
    pak_names: Option<PathBuf>,
    /// Export/Import data type
    #[arg(short = 't', long = "type", name = "TYPE", global = true, value_enum, default_value_t = Type::Banks)]
    dtype: Type,
}

#[derive(Debug, Subcommand)]
pub enum Action {
    /// Export the files from sfx archive
    #[command(arg_required_else_help = true)]
    Export { output_folder: Option<PathBuf> },
    /// Import the files into sfx archive
    #[command(arg_required_else_help = true)]
    Import {
        #[arg(value_hint = ValueHint::DirPath, value_parser = utils::is_dir)]
        files_folder: PathBuf,
        output_file: Option<PathBuf>,
    },
}

impl Action {
    const fn name(&self) -> &str {
        match self {
            Action::Export { .. } => "Export",
            Action::Import { .. } => "Import",
        }
    }
}

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum Type {
    /// Export/Import just banks
    #[default]
    Banks,
    /// Export/Import raw sound data
    RawSound,
    /// Export/Import as PC Wav
    #[cfg(feature = "pc")]
    PcWav,
    /// Export/Import as PS2 Vag
    #[cfg(feature = "ps2")]
    Ps2Vag,
    /// Export/Import as PS2 Wav
    #[cfg(all(feature = "ps2", feature = "wav"))]
    Ps2Wav,
}

impl SfxCommands {
    pub fn command(self, mut reporter: CliReporter) -> anyhow::Result<()> {
        let mut sfx = SfxManager::new(self.lookup_table, self.pak_names, &mut reporter)?;
        let archive = sfx.load(&self.input_archive, &mut reporter)?;

        reporter.info(format!("SFX action: {}", self.action.name()));

        match self.action {
            Action::Export { output_folder } => {
                let output_dir =
                    output_folder.unwrap_or_else(|| self.input_archive.with_extension(""));

                reporter.info(format!("Export type: {:?}", self.dtype));

                match self.dtype {
                    Type::Banks => {
                        archive
                            .banks()
                            .export_all_banks(output_dir, &mut reporter)?;
                    }
                    dtype => {
                        let sound_type = get_sound_type(dtype);

                        archive
                            .banks()
                            .export_all_sounds(sound_type, output_dir, &mut reporter)?;
                    }
                }

                reporter.good("Export finished.");
            }
            Action::Import {
                files_folder,
                output_file,
            } => {
                let output_file = output_file.unwrap_or_else(|| {
                    let extension = self
                        .input_archive
                        .extension()
                        .and_then(OsStr::to_str)
                        .map(ToOwned::to_owned)
                        .unwrap_or_default();
                    self.input_archive.with_extension(extension + ".new")
                });

                reporter.info(format!("Import type: {:?}", self.dtype));

                match self.dtype {
                    Type::Banks => {
                        archive.import_banks(
                            files_folder,
                            output_file,
                            &mut sfx.lookup_table,
                            &mut reporter,
                        )?;
                    }
                    dtype => {
                        let sound_type = get_sound_type(dtype);

                        archive.import_sounds(
                            sound_type,
                            files_folder,
                            output_file,
                            &mut sfx.lookup_table,
                            &mut reporter,
                        )?;
                    }
                }

                reporter.good("Import finished.");
            }
        }
        Ok(())
    }
}

fn get_sound_type(dtype: Type) -> SoundType {
    match dtype {
        Type::RawSound => SoundType::Raw,
        #[cfg(feature = "pc")]
        Type::PcWav => SoundType::PcWav,
        #[cfg(feature = "ps2")]
        Type::Ps2Vag => SoundType::Ps2Vag,
        #[cfg(all(feature = "ps2", feature = "wav"))]
        Type::Ps2Wav => SoundType::Ps2Wav,
        _ => unreachable!(),
    }
}
