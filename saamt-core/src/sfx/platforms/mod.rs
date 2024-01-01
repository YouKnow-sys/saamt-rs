//! Supported platforms to work with raw sounds.

#[cfg(feature = "pc")]
pub mod pc;
#[cfg(feature = "ps2")]
pub mod ps2;
pub mod raw;
