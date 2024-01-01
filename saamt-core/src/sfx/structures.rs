use std::fmt::Debug;

use binrw::binrw;

const MAX_SOUND_ENTRIES: usize = 400;

/// SFX bank header
#[binrw]
#[brw(little)]
#[br(import_raw(buf_len: usize))]
pub struct BankHeader {
    #[br(temp, assert(num_sounds <= MAX_SOUND_ENTRIES as u16, "Number of sound entries can't be bigger then {MAX_SOUND_ENTRIES}"))]
    #[bw(
        assert(sound_entries.len() <= MAX_SOUND_ENTRIES, "Number of sound entries can't be bigger then {MAX_SOUND_ENTRIES}"),
        calc = sound_entries.len() as u16
    )]
    pub num_sounds: u16,
    padding: u16,
    // we do the map for updating sizes after read
    #[br(count = num_sounds as usize, map = |se: Vec<SoundEntry>| generate_sizes(se, buf_len))]
    #[brw(pad_size_to = MAX_SOUND_ENTRIES * SoundEntry::SIZE)]
    pub sound_entries: Vec<SoundEntry>,
}

impl BankHeader {
    /// Size of header
    // 4 => num_sounds
    // 12 => size of SoundEntry
    pub const SIZE: usize = 4 + (MAX_SOUND_ENTRIES * SoundEntry::SIZE);
}

impl Debug for BankHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BankHeader")
            .field("num_sounds", &self.sound_entries.len())
            .field("sound_entries", &self.sound_entries)
            .finish()
    }
}

/// Compute and update entry sizes.
/// ## Parameters:
/// - `len`: size of the whole buffer.
fn generate_sizes(mut sound_entries: Vec<SoundEntry>, len: usize) -> Vec<SoundEntry> {
    let mut entries = sound_entries.iter_mut().peekable();
    while let Some(current_entry) = entries.next() {
        // get file len from next entry or if last from bank len
        let len = match entries.peek() {
            Some(next_entry) => (next_entry.offset - current_entry.offset) as usize,
            None => len - current_entry.offset as usize,
        };

        current_entry.size = len;
    }
    sound_entries
}

/// Sound entries
#[binrw] 
#[derive(Debug, Default)]
#[brw(little)]
pub struct SoundEntry {
    /// Offset of the sound inside the bank.
    pub offset: u32,
    /// Where the start of the loop is (in samples).
    // most of the times 0xFFFFFFFF
    pub loop_offset: u32,
    /// Sample rate (measured in Hz).
    pub sample_rate: u16,
    /// Audio headroom. Defines how much louder than average
    /// this sound effect is expected to go.
    pub headroom: u16,
    /// Size of the sound entry inside the buffer.
    // this is not part of SoundEntry structure, I'm adding this for managing the entry easier
    #[brw(ignore)]
    pub size: usize,
}

impl SoundEntry {
    /// Size of sound entry struct
    // We have to do this hack because we are adding `size` to struct
    // this will change the struct size
    pub const SIZE: usize = 12;

    /// Create a new `SoundEntry` based on input information.
    pub fn new(offset: u32, sample_rate: u16, headroom: u16) -> Self {
        Self {
            offset,
            loop_offset: 0xFFFFFFFF,
            sample_rate,
            headroom,
            size: 0,
        }
    }
}
