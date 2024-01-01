//! Load and modify bankslot
use std::{
    fmt::Debug,
    io::{Read, Seek, Write},
};

use binrw::{binrw, BinRead, BinWrite};

use crate::error::*;

/// # Bank Slot
/// hold all banks slots.
#[binrw]
#[brw(little)]
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BankSlot {
    #[br(temp)]
    #[bw(calc = slots.len() as _)]
    num_slots: u16,
    #[br(count = num_slots as usize)]
    pub slots: Vec<Slot>,
}

/// # Slot
#[binrw]
#[brw(little)]
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Slot {
    /// Sum of all buffer sizes before this slot (i.e. the offset).
    offset: u32,
    /// Buffer size for this slot.
    size: u32,
    /// first 2-i32: {-1, -1} on disk. Related to feet sounds?
    /// last i32: unknown
    unknown: [i32; 3],
    // I should just use [[i32; 3]; 400] but because serde doesn't support
    // const generic I'll use vector instead.
    #[br(count = 400)]
    #[bw(assert(ignored.len() == 400, "ignored field should be 4804 in lenght, but instead it was {}", ignored.len()))]
    ignored: Vec<[i32; 3]>,
}

impl BankSlot {
    /// Read and parse the [`BankSlot`] from the reader.
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        BankSlot::read(reader).map_err(Error::BinRw)
    }

    /// Write the [`BankSlot`] to the given writer.
    pub fn to_writer<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        self.write(writer).map_err(Error::BinRw)
    }

    /// Export and return all the buffer size.
    pub fn export_buf_sizes(&self) -> Vec<u32> {
        self.slots.iter().map(|s| s.size).collect()
    }

    /// Update the buffer sizes and offsets in BankSlot.
    /// ## Note:
    /// this function will panic if `sizes` len isn't the same as `slots` len.
    pub fn update_buf_sizes(&mut self, sizes: Vec<u32>) {
        assert_eq!(sizes.len(), self.slots.len());

        let mut offset = self.slots.first().unwrap().offset;
        for (slot, size) in self.slots.iter_mut().zip(sizes) {
            slot.size = size;
            slot.offset = offset;
            offset += size;
        }
    }
}

impl Debug for BankSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BankSlot")
            .field("num_slots", &self.slots.len())
            .field("slots", &self.slots)
            .finish()
    }
}

impl Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slot")
            .field("offset", &self.offset)
            .field("size", &self.size)
            .field("unknown", &self.unknown)
            // .field("ignored", &self.ignored) // we skip this section because its not useful
            .finish()
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn load_bankslot() {
        let bs = BankSlot::from_reader(&mut Cursor::new(include_bytes!(
            r"../../../test-assets/BankSlot.dat"
        )));

        assert!(bs.is_ok());
    }
}
