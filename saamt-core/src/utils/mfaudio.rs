//! A set of helper functions to do different actions using `MFAudio` tool.
//! **Note** that this module is only availible in windows platform.

use std::{os::windows::process::CommandExt, path::Path, process::Command};

use crate::error::*;

#[derive(Default, PartialEq, Eq)]
pub enum MFAudioType {
    #[default]
    Wavu,
    Vagc,
    Ss2u,
    Ss2c,
    Rawu,
    Rawc,
}

impl MFAudioType {
    const fn mfaudio_out_type(&self) -> &str {
        match self {
            MFAudioType::Wavu => "/OTWAVU",
            MFAudioType::Vagc => "/OTVAGC",
            MFAudioType::Ss2u => "/OTSS2U",
            MFAudioType::Ss2c => "/OTSS2C",
            MFAudioType::Rawu => "/OTRAWU",
            MFAudioType::Rawc => "/OTRAWC",
        }
    }
}

/// Convert the input to the [`MFAudioType`] with default settings.
pub fn convert(out_type: MFAudioType, input: &Path, output: &Path) -> Result<()> {
    let status = Command::new(r"MFAudio.exe")
        .raw_arg(out_type.mfaudio_out_type())
        .raw_arg(format!("\"{}\"", input.display()))
        .raw_arg(format!("\"{}\"", output.display()))
        .status()?;

    if !status.success() {
        return Err(Error::MFAudioConvertToWavFailed(
            status.code().unwrap_or(110),
        ));
    }

    std::fs::remove_file(input)?;

    Ok(())
}
