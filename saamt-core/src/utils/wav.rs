//! A simple wrapper for the underlying hound wav.
//! created for easier management.

use std::{
    fs::File,
    io::{BufWriter, Read, Seek, Write},
    path::Path,
};

use binrw::io::BufReader;
use hound::{WavReader, WavSpec, WavWriter};

use crate::error::*;

/// Wav audio
#[derive(Clone)]
pub struct Wav {
    pub(crate) spec: WavSpec,
    pub(crate) samples: Vec<i16>,
}

impl Wav {
    /// Read and create a Wav from input reader
    pub fn new<R: Read + Seek>(reader: R) -> Result<Self> {
        let reader = WavReader::new(reader)?;
        Ok(Self {
            spec: reader.spec(),
            samples: reader
                .into_samples()
                .collect::<std::result::Result<_, _>>()?,
        })
    }

    /// a helper method for reading the wav file from a file directly.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let reader = BufReader::new(File::open(path)?);
        Self::new(reader)
    }

    /// Return specifies properties of the audio data.
    pub fn spec(&self) -> WavSpec {
        self.spec
    }

    /// Return the samples
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Write the wav file to the input writer
    pub fn to_writer<W: Write + Seek>(&self, writer: W) -> Result<()> {
        let mut writer = WavWriter::new(writer, self.spec)?;
        let mut i16_writer = writer.get_i16_writer(self.samples.len() as _);
        self.samples
            .iter()
            .for_each(|sample| i16_writer.write_sample(*sample));

        i16_writer.flush()?;
        writer.flush()?;
        writer.finalize()?;

        Ok(())
    }

    /// Helper method to write wav file to disk directly.
    pub fn to_disc(&self, path: impl AsRef<Path>) -> Result<()> {
        let writer = BufWriter::new(File::create(path)?);
        self.to_writer(writer)
    }
}
