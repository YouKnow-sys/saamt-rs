//! A set of utils for doing different things like converting between format, encoding
//! and decoding files and etc.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

pub mod helpers;
#[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
pub mod mfaudio;
pub mod vag;
#[cfg(all(feature = "wav", any(feature = "ps2", feature = "pc")))]
pub mod wav;

/// Generate a file list from input `path`
///
/// # Inputs
/// * `path`: Path to input folder
/// * `ext`: file extension to filter files with, `None` mean no filter
/// * `depth`: depth of the search, normally you should pass [`usize::MAX`] here
/// # Return
/// This function will return a `Vec` of `PathBuf`
pub(crate) fn generate_file_list(
    path: impl AsRef<Path>,
    extension: Option<&[&str]>,
    depth: usize,
) -> Vec<PathBuf> {
    WalkDir::new(path)
        .max_depth(depth)
        .into_iter()
        .filter_map(|f| {
            let f = f.ok()?;
            if f.path().is_dir() {
                return None;
            }
            let Some(ext) = extension else {
                return Some(f.path().to_path_buf()); // No filter
            };
            let file_ext = f.path().extension().and_then(OsStr::to_str)?;
            if ext.contains(&file_ext) {
                return Some(f.path().to_path_buf());
            }
            None
        })
        .collect()
}

/// Generate a list of all folders from input `input`.
pub(crate) fn generate_folder_list(path: impl AsRef<Path>, depth: usize) -> Vec<PathBuf> {
    WalkDir::new(path)
        .max_depth(depth)
        .into_iter()
        .filter_map(|f| {
            let f = f.ok()?;
            f.path().is_dir().then(|| f.path().to_path_buf())
        })
        .collect()
}
