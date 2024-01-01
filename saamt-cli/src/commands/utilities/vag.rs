use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum, ValueHint};
use saamt_core::{
    reporter::Logger,
    utils::vag::{encoder::LoopMode as ILoopMode, VagAudio},
};

#[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
use saamt_core::utils::mfaudio::{self, MFAudioType};

use crate::{commands::utils, reporter::CliReporter};

#[derive(Debug, Parser)]
pub struct VagCommands {
    /// Input file
    #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
    input: PathBuf,
    /// Output file
    output: Option<PathBuf>,
    /// Vag action
    #[command(subcommand)]
    action: Action,
    /// Use MFAudio for converstion
    #[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
    #[arg(
        short,
        long,
        alias = "mfaudio",
        long_help = "Use MFAudio for converstion, make sure to place the MFAudio exe next to program"
    )]
    use_mfaudio: bool,
}

#[derive(Clone, Debug, Subcommand)]
enum Action {
    /// Export and decode the vag into wav
    #[command(alias = "decode")]
    ToWav,
    /// Encode a Wav file into Vag
    #[command(arg_required_else_help = true, alias = "encode")]
    ToVag {
        /// What loop mode to use when encoding wav to vag
        #[arg(short = 'o', long, value_enum, default_value_t = LoopMode::FromInput)]
        loop_mode: LoopMode,
    },
}

/// What loop mode to use when encoding wav to vag
#[derive(Clone, Copy, Debug, Default, ValueEnum, PartialEq, Eq)]
enum LoopMode {
    /// Check the input wav file for smpl chunk and use that for looping.
    #[default]
    FromInput,
    /// Force Loop
    ForceLoop,
    /// Force No Loop
    ForceNoLoop,
}

impl From<LoopMode> for ILoopMode {
    fn from(val: LoopMode) -> Self {
        match val {
            LoopMode::FromInput => Self::FromInput,
            LoopMode::ForceLoop => Self::ForceLoop,
            LoopMode::ForceNoLoop => Self::ForceNoLoop,
        }
    }
}

impl Action {
    const fn extension(&self) -> &'static str {
        match self {
            Action::ToWav => "wav",
            Action::ToVag { .. } => "vag",
        }
    }
}

impl VagCommands {
    pub fn command(self, mut reporter: CliReporter) -> anyhow::Result<()> {
        let output = self
            .output
            .unwrap_or_else(|| self.input.with_extension(self.action.extension()));

        reporter.info(format!("Vag utility action: {:?}", self.action));

        match self.action {
            Action::ToWav => {
                #[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
                if self.use_mfaudio {
                    if !std::path::Path::new(r"MFAudio.exe").is_file() {
                        anyhow::bail!("Can't find MFAudio next to the tool.");
                    }

                    reporter.info("Starting converstion using mfaudio.");
                    mfaudio::convert(MFAudioType::Wavu, &self.input, &output)?;
                    reporter.good("Converstion finished.");

                    return Ok(());
                }

                reporter.info("Opening Vag file.");
                let vag = VagAudio::from_file(self.input)?;
                reporter.good("Vag file loaded.");

                reporter.info("Decoding and saving Vag to Wav and save it to disk.");
                vag.to_wav().to_disc(output)?;
                reporter.good("Wav saved to disk.");
            }
            Action::ToVag { loop_mode } => {
                #[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
                if self.use_mfaudio {
                    if !std::path::Path::new(r"MFAudio.exe").is_file() {
                        anyhow::bail!("Can't find MFAudio next to the tool.");
                    }

                    reporter.info("Starting converstion using mfaudio.");
                    mfaudio::convert(MFAudioType::Vagc, &self.input, &output)?;
                    reporter.good("Converstion finished.");

                    return Ok(());
                }

                reporter.info("Opening Wav file.");
                let vag = VagAudio::from_wav(self.input, loop_mode.into())?;
                reporter.good("Wav file loaded.");

                reporter.info("Encoding and saving Wav to Vag and save it to disk.");
                vag.to_disk(output)?;
                reporter.good("Vag saved to disk.");
            }
        }

        Ok(())
    }
}
