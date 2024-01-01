use std::{
    io::{Cursor, Seek, Write},
    path::Path,
};

#[cfg(feature = "wav")]
use crate::utils::wav::Wav;
use crate::{
    error::*,
    sfx::{
        bank::{Bank, Banks},
        sound::{RawSound, RawSounds},
        structures::SoundEntry,
    },
    utils::{
        helpers::DataSaveAll,
        vag::{encoder::LoopMode, Vag, VagAudio},
    },
};

#[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
use crate::reporter::Logger;
#[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
use crate::reporter::{ProgressReport, ProgressReporterIterator};

/// Imports a VAG audio file from the given path into the provided
/// SoundEntry and bytes writer. sets the sample rate and size on
/// the SoundEntry, and writes the VAG raw bytes to the writer.
/// Returns false to indicate the sound is mono.
pub fn import_vag(
    path: &Path,
    sentry: &mut SoundEntry,
    bytes_writer: &mut Cursor<Vec<u8>>,
) -> Result<bool> {
    let vag = VagAudio::from_file(path)?;
    let vag_bytes = vag.raw_vag_bytes();

    sentry.sample_rate = vag.0.sample_rate as _;
    sentry.size = vag_bytes.len();

    bytes_writer.write_all(&vag_bytes)?;

    Ok(false)
}

/// Imports a WAV audio file from the given path into the provided
/// SoundEntry and bytes writer. Sets the sample rate and size on
/// the SoundEntry, encodes the WAV to VAG format and writes the VAG
/// raw bytes to the writer.
/// Returns false to indicate the sound is mono.
#[cfg(feature = "wav")]
pub fn import_wav(
    path: &Path,
    sentry: &mut SoundEntry,
    bytes_writer: &mut Cursor<Vec<u8>>,
) -> Result<bool> {
    let vag = VagAudio::from_wav(path, LoopMode::FromInput)?;
    let vag_bytes = vag.raw_vag_bytes();

    sentry.sample_rate = vag.0.sample_rate as _;
    sentry.size = vag_bytes.len();

    bytes_writer.write_all(&vag_bytes)?;

    Ok(false)
}

/// Iterator over raw sounds converted to PS2 VAG format.
///
/// Wraps a `RawSounds` iterator and converts each raw sound to PS2 VAG
/// when iterating. This allows iterating over sounds in PS2 VAG format
/// without having to do the conversion upfront.
pub struct PS2Sounds<'a>(RawSounds<'a>);

impl<'a> Iterator for PS2Sounds<'a> {
    type Item = VagAudio;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|rs| rs.as_ps2_vag())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> ExactSizeIterator for PS2Sounds<'a> {}

impl<'a> DataSaveAll for PS2Sounds<'a> {
    fn fullname(index: usize) -> String {
        format!("sound_{index:03}.vag")
    }

    fn write<W: Write + Seek>(data: Self::Item, writer: &mut W) -> Result<()> {
        data.to_writer(writer)
    }
}

impl<'a> From<RawSounds<'a>> for PS2Sounds<'a> {
    fn from(value: RawSounds<'a>) -> Self {
        PS2Sounds(value)
    }
}

impl Banks {
    /// Convert all the vag to wav and save them to disk using mfaudio.
    ///
    /// ## Note:
    /// remember you have to put `MFAudio.exe` next to program for this function to work.
    #[cfg(all(target_os = "windows", feature = "ps2-export-mfaudio"))]
    pub fn export_sounds_as_wav_mfaudio_ps2(
        self,
        ouput_dir: impl AsRef<Path>,
        reporter: &mut (impl ProgressReport + Logger),
    ) -> Result<()> {
        use std::{any::Any, path::PathBuf, sync::mpsc::channel};

        enum Action {
            /// Push a vag file to convert to wav and save in given path
            PushFile(VagAudio, PathBuf),
            /// Finish action, there is nothing more todo
            Finish,
        }

        fn get_err_msg(e: Box<dyn Any + Send>) -> String {
            match (e.downcast_ref(), e.downcast_ref::<String>()) {
                (Some(&s), _) => s,
                (_, Some(s)) => &**s,
                _ => "<No panic message>",
            }
            .to_owned()
        }

        // check if MFAudio exist or not
        if !std::env::current_dir()?.join("MFAudio.exe").is_file() {
            return Err(Error::NoMFAudioFound);
        }

        let (sender, handle) = {
            let (sender, receiver) = channel::<Action>();

            let handle = std::thread::spawn(move || -> Result<()> {
                loop {
                    match receiver.try_recv() {
                        Ok(action) => match action {
                            Action::PushFile(vag, path) => vag.save_as_wav_mfaudio(path)?,
                            Action::Finish => break,
                        },
                        Err(e) => match e {
                            std::sync::mpsc::TryRecvError::Empty => (),
                            std::sync::mpsc::TryRecvError::Disconnected => {
                                panic!("Worker thread channel disconnected")
                            }
                        },
                    }
                }

                Ok(())
            });

            (sender, handle)
        };

        let output_dir = ouput_dir.as_ref();

        for bank in self.banks_iter() {
            let bank = bank?;
            let name = format!("bank_{:03}", bank.index);

            let output_dir = output_dir.join(&name);
            if !output_dir.is_dir() {
                std::fs::create_dir_all(&output_dir)?;
            }

            for raw_sound in
                bank.raw_sounds()
                    .progress_report(reporter, bank.header.sound_entries.len(), name)
            {
                let sname = format!("sound_{:03}", raw_sound.index);
                let sound = create_vag_audio(raw_sound.bytes, raw_sound.sample_rate as _, &sname);

                let wav_path = output_dir.join(sname + ".wav");

                let Ok(_) = sender.send(Action::PushFile(sound, wav_path)) else {
                    if handle.is_finished() {
                        return match handle.join() {
                            Ok(r) => Err(Error::WavWorkerThreadError(r.unwrap_err().to_string())),
                            Err(e) => Err(Error::WavWorkerThreadError(get_err_msg(e))),
                        };
                    }
                    return Err(Error::WavWorkerThreadError(
                        "sending on a closed channel".to_owned(),
                    ));
                };
            }
        }

        let Ok(_) = sender.send(Action::Finish) else {
            if handle.is_finished() {
                return match handle.join() {
                    Ok(r) => Err(Error::WavWorkerThreadError(r.unwrap_err().to_string())),
                    Err(e) => Err(Error::WavWorkerThreadError(get_err_msg(e))),
                };
            }
            return Err(Error::WavWorkerThreadError(
                "sending on a closed channel".to_owned(),
            ));
        };

        // wait for the worker thread to finish working
        reporter.info("Waiting for MFAudio to finish converting.");
        if let Err(e) = handle.join() {
            return Err(Error::WavWorkerThreadError(get_err_msg(e)));
        }
        reporter.good("All audio converted to wav.");

        Ok(())
    }
}

impl Bank {
    /// Converts the raw sounds in this bank to PS2 VAG sounds.
    ///
    /// This should only be used if you are certain all sounds in the bank are
    /// from the PS2 version of the game. Otherwise, use
    /// `raw_sounds()` to get the raw sounds before converting.
    pub fn ps2_sounds(&self) -> PS2Sounds {
        self.raw_sounds().into()
    }
}

// PS2 related functions
impl<'a> RawSound<'a> {
    /// Converts the raw sound to a PS2 VAG audio format.
    ///
    /// This does not validate if the input is a valid VAG file.
    /// It simply converts the raw bytes and sample rate to a VAG struct.
    /// The VAG struct can then be further processed and validated.
    pub fn as_ps2_vag(&self) -> VagAudio {
        let name = format!("sound_{:03}", self.index);
        create_vag_audio(self.bytes, self.sample_rate as u32, &name)
    }

    /// Converts the raw PS2 sound to a WAV audio format.
    ///
    /// This converts the sound to VAG format first,
    /// then converts the VAG data to WAV.
    ///
    /// Requires the `wav` feature to be enabled.
    #[cfg(feature = "wav")]
    pub fn as_ps2_wav(&self) -> Wav {
        self.as_ps2_vag().to_wav()
    }
}

fn create_vag_audio(bytes: &[u8], sample_rate: u32, sname: &str) -> VagAudio {
    let mut name = [0_u8; 16];
    name[0..9].copy_from_slice(sname.as_bytes());

    Vag::new(sample_rate, name, bytes.to_owned()).into()
}
