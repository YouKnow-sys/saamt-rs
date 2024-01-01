//! Error types of [saamt-core](`crate`)

/// The main result type of [saamt-core](`crate`)
pub type Result<T> = std::result::Result<T, Error>;

/// The main error type of [saamt-core](`crate`)
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    BinRw(#[from] binrw::Error),

    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),

    #[cfg(feature = "wav")]
    #[error(transparent)]
    Wav(#[from] hound::Error),

    #[error("No {0} file found in input folder")]
    NoFileFound(&'static str),

    #[error("No {0} folder found in input folder")]
    NoFolderFound(&'static str),

    #[error("Can't get the basename of \"{0}\"")]
    CantGetBaseName(String),

    #[error("Couldn't match file name with a valid soundbank")]
    CantFindInLookupTable,

    #[error("Unknown lookup file, keep the lookup file name same as default")]
    UnknownLookupFile,

    #[error("No entry match pak index in the lookup file")]
    NoEntryMatch,

    #[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
    #[error("Can't find \"MFAudio.exe\" beside the program")]
    NoMFAudioFound,

    #[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
    #[error("Failed to convert the vag audio to wav using \"MFAudio.exe\", MFAudio returned {0}")]
    MFAudioConvertToWavFailed(i32),

    #[error("Unsorted sfx banks, tool expect the bank entries to be back to back")]
    UnsortedSfxBanks,

    #[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
    #[error("There was a error in wav worker thread: {0}")]
    WavWorkerThreadError(String),

    #[cfg(feature = "wav")]
    #[error("Invalid wav file: {0}")]
    InvalidWav(String),

    #[cfg(feature = "wav")]
    #[error("Found an invalid sound data when trying to convert to wav")]
    InvalidWavSoundData,

    #[error("Can't find index in Lookup Table")]
    CantFindIndexInLookUpTable,
}
