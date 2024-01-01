// This is a poor rewrite of https://github.com/eurotools/es-ps2-vag-tool/blob/main/PS2VagTool/Vag%20Functions/SonyVagEncoder.cs
// I mostly just followed what the C# version do, C# version work but its super inefficient, so is my rewrite...
// there are so many un-needed allocation that can be avoided, I may check it later, but not for now... but you know, just in case "TODO".

use std::{
    ffi::OsStr,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use hound::{read_wave_header, WavReader, WavSpec};

use crate::{
    error::*,
    utils::vag::{VAGChunk, VAG_SAMPLE_BYTES},
};

use super::{PackInfo, VAGFlag, Vag, VagAudio, VAG_SAMPLE_NIBBL};

const VAG_LUT_ENCODER: [[f64; 2]; 5] = [
    [0.0, 0.0],
    [-60.0 / 64.0, 0.0],
    [-115.0 / 64.0, 52.0 / 64.0],
    [-98.0 / 64.0, 55.0 / 64.0],
    [-122.0 / 64.0, 60.0 / 64.0],
];

/// Different available loop modes.
#[derive(Default, PartialEq, Eq)]
pub enum LoopMode {
    /// Check the input wav file for smpl chunk and use that for looping.
    #[default]
    FromInput,
    /// Force Loop
    ForceLoop,
    /// Force No Loop
    ForceNoLoop,
}

#[derive(Default)]
struct IteratorData {
    idx: usize,
    pos: usize,
    hist_0_1: f64,
    hist_0_2: f64,
    hist_1_1: f64,
    hist_1_2: f64,
    last_pack_info: Option<PackInfo>,
    quit_at_the_next_iteration: bool,
}

/// An encoder that is able to encode wav samples to VagChunks.
pub struct WAV2VAGEncoder {
    name: String,
    spec: WavSpec,
    samples: Vec<i16>,
    loop_start_end: (usize, usize),
    use_loop: bool,
    iter_data: IteratorData,
}

impl WAV2VAGEncoder {
    /// Create a new wav encoder that will encode wav samples to vag
    /// keep in mind that we only support mono files and PCM.
    pub fn new(wav_path: &Path, loop_mode: LoopMode) -> Result<Self> {
        let mut wav_reader = BufReader::new(File::open(wav_path)?);

        if let Err(error) = read_wave_header(&mut wav_reader) {
            return Err(Error::InvalidWav(error.to_string()));
        }

        let loop_start_end = match try_read_sample_chunk(&mut wav_reader) {
            Ok(Some((ld1, ld2))) => (
                get_loop_offset(ld1).wrapping_sub(1) as usize,
                get_loop_offset(ld2).wrapping_sub(2) as usize,
            ),
            _ => (0, usize::MAX),
        };

        // seek back to start of wav because we want to parse it again
        wav_reader.seek(SeekFrom::Start(0))?;

        let wav = WavReader::new(wav_reader)?;

        let spec: WavSpec = wav.spec();

        if spec.channels != 1 {
            return Err(Error::InvalidWav(
                "Wav with more then one channels aren't supported".to_owned(),
            ));
        }

        let mut samples: Vec<i16> = wav.into_samples().collect::<std::result::Result<_, _>>()?;

        // make sure that samples is in pow of `VAG_SAMPLE_NIBBL`
        let rs = samples.len() % VAG_SAMPLE_NIBBL;
        if rs != 0 {
            samples.extend(vec![0; rs]);
        }

        Ok(Self {
            name: wav_path
                .with_extension("")
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or_default()
                .to_owned(),
            spec,
            samples,
            loop_start_end,
            use_loop: matches!(loop_mode, LoopMode::FromInput | LoopMode::ForceLoop),
            iter_data: IteratorData::default(),
        })
    }

    /// Create an encoder for the samples of input wav file and return the spec of it for later uses.
    ///
    /// keep in mind that the iterator **can** fail, in that case it will just finish early without
    /// encoding every sample.
    pub fn encoder(self) -> (SampleEncoder, WavSpec) {
        (
            SampleEncoder {
                samples: self.samples,
                loop_start_end: self.loop_start_end,
                use_loop: self.use_loop,
                iter_data: self.iter_data,
            },
            self.spec,
        )
    }

    /// encode the wav file and generate a [`VagAudio`] from it.
    pub fn generate_vag(self) -> VagAudio {
        let mut name = [0; 16];
        let name_str = self.name.clone();
        if name_str.len() >= 16 {
            name[0..16].copy_from_slice(&name_str.as_bytes()[0..16]);
        } else if !name_str.is_empty() {
            name[0..name_str.len()].copy_from_slice(&name_str.as_bytes()[0..name_str.len()]);
        }

        let (encoder, spec) = self.encoder();

        let chunks: Vec<VAGChunk> = encoder.collect();

        let vag = Vag::new_from_chunks(spec.sample_rate, name, chunks);

        vag.into()
    }
}

/// Sample encoder
pub struct SampleEncoder {
    samples: Vec<i16>,
    loop_start_end: (usize, usize),
    use_loop: bool,
    iter_data: IteratorData,
}

impl Iterator for SampleEncoder {
    type Item = VAGChunk;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_data.quit_at_the_next_iteration {
            return self.iter_data.last_pack_info.take().map(|last_pack_info| {
                // put terminating chunk
                VAGChunk {
                    pack_infos: last_pack_info,
                    flags: VAGFlag::PlaybackEnd,
                    sample: Default::default(),
                }
            });
        }

        // get chunk data
        let samples: [i16; VAG_SAMPLE_NIBBL] = self
            .samples
            .get(self.iter_data.pos..self.iter_data.pos + VAG_SAMPLE_NIBBL)?
            .try_into()
            .unwrap(); // we are sure that we have VAG_SAMPLE_NIBBL here so we just unwrap

        let mut chunk = VAGChunk::default();

        // get predict and shift
        let (predict, shift, d_samples) = self.get_predict_and_shift(&samples);

        chunk.pack_infos.update_predict(predict as i8);
        chunk.pack_infos.update_shift_factor(shift);

        // get flag
        chunk.flags = self.get_flags();

        // pack
        let mut out_buf = [0i16; VAG_SAMPLE_NIBBL];
        out_buf.iter_mut().zip(d_samples).for_each(|(o, s)| {
            let s_trans = s
                + self.iter_data.hist_1_1 * VAG_LUT_ENCODER[predict][0]
                + self.iter_data.hist_1_2 * VAG_LUT_ENCODER[predict][1];
            let s = (s_trans * (1 << shift) as f64) as i32;
            let mut sample =
                (((s + 0x800) as u32 & 0xFFFFF000) as i32).clamp(i16::MIN as _, i16::MAX as _);

            *o = sample as i16;

            sample >>= shift;

            self.iter_data.hist_1_2 = self.iter_data.hist_1_1;
            self.iter_data.hist_1_1 = sample as f64 - s_trans;
        });

        for i in 0..VAG_SAMPLE_BYTES {
            chunk.sample[i] =
                (((out_buf[(i * 2) + 1] >> 8) & 0xf0) | ((out_buf[i * 2] >> 12) & 0xf)) as u8;
        }

        if !self.use_loop {
            self.iter_data.last_pack_info = Some(chunk.pack_infos);
        }

        self.iter_data.idx += 1;
        self.iter_data.pos += VAG_SAMPLE_NIBBL;

        Some(chunk)
    }
}

impl SampleEncoder {
    fn get_predict_and_shift(&mut self, samples: &[i16]) -> (usize, i8, [f64; VAG_SAMPLE_NIBBL]) {
        // find predict
        let mut predict = 0;
        let mut min = 1e10;
        let mut s_1 = 0.0;
        let mut s_2 = 0.0;
        let mut predict_buf = [[0.0; 5]; VAG_SAMPLE_NIBBL];

        for (i, vle) in VAG_LUT_ENCODER.iter().enumerate() {
            let mut max = 0.0;

            s_1 = self.iter_data.hist_0_1;
            s_2 = self.iter_data.hist_0_2;

            for n in 0..VAG_SAMPLE_NIBBL {
                let sample = (samples[n] as f64).clamp(-30720.0, 30719.0);

                let ds = sample + s_1 * vle[0] + s_2 * vle[1];
                predict_buf[n][i] = ds;

                let abs = ds.abs();
                if abs > max {
                    max = abs;
                }

                s_2 = s_1;
                s_1 = sample;
            }

            if max < min {
                min = max;
                predict = i;
            }

            if min <= 7.0 {
                predict = 0;
                break;
            }
        }

        // store s[t-2] and s[t-1] in a static variable
        // these than used in the next function call
        self.iter_data.hist_0_1 = s_1;
        self.iter_data.hist_0_2 = s_2;

        let mut d_samples = [0.0; VAG_SAMPLE_NIBBL];
        d_samples
            .iter_mut()
            .zip(predict_buf)
            .for_each(|(ds, pb)| *ds = pb[predict]);

        // find shift
        let min = min as i32;
        let mut shift_mask = 0x4000;
        let mut shift = 0;

        while shift < 12 {
            if shift_mask & (min + (shift_mask >> 3)) != 0 {
                break;
            }
            shift += 1;
            shift_mask >>= 1;
        }

        (predict, shift as i8, d_samples)
    }

    fn get_flags(&mut self) -> VAGFlag {
        let mut flag = VAGFlag::Nothing;
        if self.samples.len() - self.iter_data.pos > VAG_SAMPLE_NIBBL {
            if self.use_loop {
                flag = VAGFlag::LoopRegion;
                if self.iter_data.idx == self.loop_start_end.0 {
                    flag = VAGFlag::LoopStart;
                }
                if self.iter_data.idx == self.loop_start_end.1 {
                    flag = VAGFlag::LoopEnd;
                    self.iter_data.quit_at_the_next_iteration = true;
                }
            }
        } else {
            flag = VAGFlag::LoopLastBlock;
            if self.use_loop {
                flag = VAGFlag::LoopEnd;
            }
        }

        flag
    }
}

// this function doesn't check if the file is a valid wav or not.
// this is not a good way, because in this way we are reading the
// wav file two time, but I really don't want to change how hound
// work at the moment...
fn try_read_sample_chunk(reader: &mut BufReader<File>) -> Result<Option<(u32, u32)>> {
    use binrw::BinRead;

    let mut chunk_id = [0_u8; 4];

    while reader.read_exact(&mut chunk_id).is_ok() {
        let len = u32::read_le(reader)?;

        if &chunk_id == b"smpl" {
            reader.seek_relative(12)?;
            let _midi_note = i32::read_le(reader)?;
            reader.seek_relative(16)?;
            let number_of_samples = i32::read_le(reader)?;
            reader.seek_relative(8)?;

            let mut loop_info = (0, 0);
            for _ in 0..number_of_samples {
                // Read Chunk info
                let _cue_point_id = i32::read_le(reader)?;
                let _loop_type = i32::read_le(reader)?; // 0 = loop forward, 1 = alternating loop, 2 = reverse

                let start = u32::read_le(reader)?;
                let end = u32::read_le(reader)?;
                let _fraction = i32::read_le(reader)?;
                let _play_count = i32::read_le(reader)?;

                // Save Data
                loop_info = (start, end);
            }

            return Ok(Some(loop_info));
        } else {
            reader.seek_relative(len as i64)?;
        }
    }

    Ok(None)
}

fn get_loop_offset(loop_offset: u32) -> u32 {
    loop_offset / 28 + if loop_offset % 28 != 0 { 2 } else { 1 }
}
