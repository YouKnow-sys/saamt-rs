//! Get pak names either from dat files or just the default built-in ones

use std::{
    io::{Read, Seek},
    ops::Deref,
};

use binrw::{helpers::until_eof, BinRead, NullString};

use crate::error::*;

pub mod names;

/// fix for PS2 names
macro_rules! fix_ps2_name {
    ($name:expr) => {
        $name.trim_end_matches(['0', '1', '2']).to_uppercase()
    };
}

/// Store all the pak names, support both `sfx` and `stream`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PakNames(Vec<String>);

impl PakNames {
    /// Try to get the default archive names based on name, support all sfx and stream names.
    pub fn try_get_defaults(name: &str) -> Option<Self> {
        let name = fix_ps2_name!(name);

        names::SFX_DEFAULT_PAK_NAMES
            .contains(&name.as_str())
            .then(Self::sfx)
            .or_else(|| {
                names::STREAM_DEFAULT_PAK_NAMES
                    .contains(&name.as_str())
                    .then(Self::stream)
            })
    }

    /// Default pak names for sfx archives.
    pub fn sfx() -> Self {
        Self(
            names::SFX_DEFAULT_PAK_NAMES
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
    }

    /// Default pak names for stream archives.
    pub fn stream() -> Self {
        Self(
            names::STREAM_DEFAULT_PAK_NAMES
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
    }

    /// read sfx pak names from reader (`PakFiles.dat`)
    pub fn sfx_from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        #[derive(BinRead)]
        #[brw(little)]
        pub struct SfxPakName {
            #[br(align_after = 52)]
            name: NullString,
        }

        let names: Vec<SfxPakName> = until_eof(reader, binrw::Endian::Little, ())?;
        let names: Vec<String> = names
            .into_iter()
            .map(|p| String::try_from(p.name))
            .collect::<std::result::Result<_, _>>()?;

        assert!(
            names.len() <= u8::MAX as usize,
            "we dont support sfx lookup files that have more then {} name",
            u8::MAX
        );

        Ok(Self(names))
    }

    /// read stream pak names from reader (`StrmPaks.dat`)
    pub fn stream_from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        #[derive(BinRead)]
        #[brw(little)]
        pub struct StreamPakName {
            #[br(align_after = 16)]
            name: NullString,
        }

        let names: Vec<StreamPakName> = until_eof(reader, binrw::Endian::Little, ())?;
        let names: Vec<String> = names
            .into_iter()
            .map(|p| String::try_from(p.name))
            .collect::<std::result::Result<_, _>>()?;

        assert!(
            names.len() <= u8::MAX as usize,
            "we dont support stream lookup files that have more then {} name",
            u8::MAX
        );

        Ok(Self(names))
    }

    /// Try to load the lookup file, we will try to detect the lookup type from the name.
    pub fn from_reader<R: Read + Seek>(name: &str, reader: &mut R) -> Result<Self> {
        let name = fix_ps2_name!(name);

        match name.as_str() {
            "PAKFILES" => Self::sfx_from_reader(reader),
            "STRMPAKS" => Self::stream_from_reader(reader),
            _ => Err(Error::UnknownLookupFile),
        }
    }

    /// Try to get a pak index matching given name; name shouldn't have extension.
    pub fn get_pak_idx_from_name(&self, name: &str) -> Option<u8> {
        if name.is_empty() {
            return None;
        }
        let name = fix_ps2_name!(name);
        self.0.iter().position(|n| n == &name).map(|n| n as u8)
    }
}

impl Deref for PakNames {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn sfx_builtin() {
        let pak_names = PakNames::sfx();

        assert_eq!(Some(0), pak_names.get_pak_idx_from_name("FEET"));
        assert_eq!(Some(0), pak_names.get_pak_idx_from_name("feet"));
        assert_eq!(Some(8), pak_names.get_pak_idx_from_name("SPc_Pa"));
        assert_eq!(None, pak_names.get_pak_idx_from_name("Something"));
        assert_eq!(None, pak_names.get_pak_idx_from_name(""));
    }

    #[test]
    fn stream_builtin() {
        let pak_names = PakNames::stream();

        assert_eq!(Some(0), pak_names.get_pak_idx_from_name("AA"));
        assert_eq!(Some(0), pak_names.get_pak_idx_from_name("aa"));
        assert_eq!(Some(16), pak_names.get_pak_idx_from_name("TK"));
        assert_eq!(None, pak_names.get_pak_idx_from_name("Something"));
        assert_eq!(None, pak_names.get_pak_idx_from_name(""));
    }

    #[test]
    fn sfx_from_reader() {
        let pak_names = PakNames::sfx_from_reader(&mut Cursor::new(include_bytes!(
            "../../../test-assets/PakFiles.dat"
        )));

        assert!(pak_names.is_ok_and(|v| v == PakNames::sfx()));
    }

    #[test]
    fn stream_from_reader() {
        let pak_names = PakNames::stream_from_reader(&mut Cursor::new(include_bytes!(
            "../../../test-assets/StrmPaks.dat"
        )));

        assert!(pak_names.is_ok_and(|v| v == PakNames::stream()));
    }
}
