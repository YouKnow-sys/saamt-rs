use std::{
    fs::File,
    io::{BufWriter, Seek, Write},
    path::Path,
};

use crate::{
    error::*,
    reporter::{ProgressReport, ProgressReporterIterator},
};

/// A helper trait to save all data inside a [`ExactSizeIterator`] to output folder.
pub trait DataSaveAll: Sized + ExactSizeIterator {
    fn fullname(index: usize) -> String;
    fn write<W: Write + Seek>(data: Self::Item, writer: &mut W) -> Result<()>;
    /// Save all the remaining data (the ones that we not already read) to the `output_dir`.
    fn save_all(
        self,
        output_dir: impl AsRef<Path>,
        reporter: &mut impl ProgressReport,
    ) -> Result<()> {
        let output_dir = output_dir.as_ref();

        if !output_dir.is_dir() {
            std::fs::create_dir_all(output_dir)?;
        }

        let len = self.len();
        for (index, data) in self
            .progress_report(reporter, len, "Saving data".to_owned())
            .enumerate()
        {
            let mut writer = BufWriter::new(File::create(output_dir.join(Self::fullname(index)))?);
            Self::write(data, &mut writer)?;
            writer.flush()?;
        }

        Ok(())
    }
}
