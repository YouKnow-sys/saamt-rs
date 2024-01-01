//! Load and modify LookupTable.

use std::{
    fmt::Debug,
    io::{Read, Seek, Write},
    ops::Deref,
};

use binrw::{helpers::until_eof, BinRead, BinWrite};

use crate::error::*;

/// ## LookupTable
/// GTASA lookup table, hold all LookupEntries.
#[derive(Debug, Clone, BinRead, BinWrite)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[brw(little)]
pub struct LookUpTable {
    #[br(parse_with = until_eof)]
    entries: Vec<LookUpEntry>,
}

/// ## LookupEntry
/// LookupEntry represents an entry in the lookup table. It contains the index,
/// offset and length for the data.
#[derive(Debug, Clone, Copy, BinRead, BinWrite)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[brw(little)]
pub struct LookUpEntry {
    pub index: u8,
    padding: [u8; 3],
    pub offset: u32,
    pub length: u32,
}

impl LookUpTable {
    /// Read and parse the [`LookUpTable`] from the reader.
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        LookUpTable::read(reader).map_err(Error::BinRw)
    }

    /// Write the [`LookUpTable`] to the given writer.
    pub fn to_writer<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        self.write(writer).map_err(Error::BinRw)
    }

    /// Get a mutable reference to a index inside the [`LookUpTable`].
    pub fn get_mut(&mut self, index: usize) -> Option<&mut LookUpEntry> {
        self.entries.get_mut(index)
    }

    /// Count how many entry inside the [`LookUpTable`] match the given index.
    pub fn count_entries_matching_pak_idx(&self, idx: u8) -> usize {
        self.entries.iter().filter(|e| e.index == idx).count()
    }

    /// Get a list of all matching entries in lookup table and index of them.
    pub fn matching_entries(&self, idx: u8) -> Vec<(usize, LookUpEntry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|&(_, e)| (e.index == idx))
            .map(|(i, e)| (i, e.to_owned()))
            .collect()
    }
}

impl Deref for LookUpTable {
    type Target = Vec<LookUpEntry>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;
    use crate::config::paknames::PakNames;

    #[test]
    fn from_reader_sfx_banks() {
        let lookup_tbl = LookUpTable::from_reader(&mut Cursor::new(include_bytes!(
            "../../../test-assets/BankLkup.dat"
        )));

        let pak_name = PakNames::sfx().get_pak_idx_from_name("FEET").unwrap();
        assert!(lookup_tbl.is_ok_and(|tbl| tbl.count_entries_matching_pak_idx(pak_name) != 0));
    }

    #[test]
    fn from_reader_stream_tracks() {
        let lookup_tbl = LookUpTable::from_reader(&mut Cursor::new(include_bytes!(
            "../../../test-assets/TrakLkup.dat"
        )));

        let pak_name = PakNames::stream().get_pak_idx_from_name("AA").unwrap();
        assert!(lookup_tbl.is_ok_and(|tbl| tbl.count_entries_matching_pak_idx(pak_name) != 0));
    }
}
