use std::{
    io::{Cursor, Seek, Write},
    path::Path,
};

use crate::{
    error::*,
    sfx::{sound::RawSounds, structures::SoundEntry},
    utils::helpers::DataSaveAll,
};

/// Imports raw PCM audio data from the given file path into the provided
/// SoundEntry and bytes writer.
/// We wont update sample rate, only size, user will have to take care of that.
pub fn import_raw(
    path: &Path,
    sentry: &mut SoundEntry,
    bytes_writer: &mut Cursor<Vec<u8>>,
) -> Result<bool> {
    let buf = std::fs::read(path)?;

    // we no longer update the file sample rate or size, we expect user to
    // take care of that.
    sentry.size = buf.len();

    bytes_writer.write_all(&buf)?;

    Ok(false)
}

impl<'a> DataSaveAll for RawSounds<'a> {
    fn fullname(index: usize) -> String {
        format!("sound_{index:03}.raw")
    }

    fn write<W: Write + Seek>(data: Self::Item, writer: &mut W) -> Result<()> {
        data.to_writer(writer)
    }
}
