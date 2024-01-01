// Just a direct rewrite of https://github.com/eurotools/es-ps2-vag-tool so don't think that much of it
// there is room for lots of improvement, but meh, Im not going to put more time to this...

use super::{VAGFlag, VagAudio, VAG_SAMPLE_NIBBL};
#[cfg(feature = "wav")]
use crate::utils::wav::Wav;

const VAG_LUT_DECODER: [[f64; 2]; 5] = [
    [0.0, 0.0],
    [60.0 / 64.0, 0.0],
    [115.0 / 64.0, -52.0 / 64.0],
    [98.0 / 64.0, -55.0 / 64.0],
    [122.0 / 64.0, -60.0 / 64.0],
];

/// A decoder that decode VAG Chunks to PCM samples
pub struct VAG2WAVDecoder<'a> {
    vag: &'a VagAudio,
    chunk_idx: usize,
    hist_1: f64,
    hist_2: f64,
}

impl<'a> VAG2WAVDecoder<'a> {
    pub fn new(vag: &'a VagAudio) -> Self {
        Self {
            vag,
            chunk_idx: 0,
            hist_1: 0.0,
            hist_2: 0.0,
        }
    }

    /// Create a decoder from the samples inside the input vag file.
    pub fn decoder(self) -> SampleDecoder<'a> {
        SampleDecoder(self)
    }

    /// Decode all samples and return them as a vector of samples
    pub fn to_decoded(self) -> Vec<i16> {
        self.decoder().flatten().collect()
    }

    /// Decode all samples inside the vag and create a wav from it.
    #[cfg(feature = "wav")]
    pub fn to_wav(self) -> Wav {
        use hound::{SampleFormat, WavSpec};

        let spec = WavSpec {
            channels: if self.vag.0.channels == 0 {
                1
            } else {
                self.vag.0.channels
            },
            sample_rate: self.vag.0.sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let samples: Vec<i16> = self.decoder().flatten().collect();

        Wav { samples, spec }
    }
}

pub struct SampleDecoder<'a>(VAG2WAVDecoder<'a>);

impl<'a> Iterator for SampleDecoder<'a> {
    type Item = [i16; VAG_SAMPLE_NIBBL];

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.0.vag.0.chunks.get(self.0.chunk_idx)?;
        self.0.chunk_idx += 1;

        if chunk.flags == VAGFlag::PlaybackEnd {
            return None;
        }

        let mut samples = [0; VAG_SAMPLE_NIBBL];

        for (i, sample) in chunk.sample.into_iter().enumerate() {
            samples[i * 2] = (sample & 0xF) as i32;
            samples[i * 2 + 1] = (sample >> 4) as i32;
        }

        let samples = samples.map(|sample| {
            // shift 4 bits to top range of i16
            let mut sample = sample << 12;
            if (sample & 0x8000) != 0 {
                sample = (sample as u32 | 0xFFFF0000) as i32;
            }

            // don't overflow the LUT array access; limit the max allowed index
            let predict = chunk
                .pack_infos
                .predict()
                .min((VAG_LUT_DECODER.len() - 1) as i8) as usize;

            let sample = (sample >> chunk.pack_infos.shift_factor()) as f64
                + self.0.hist_1 * VAG_LUT_DECODER[predict][0]
                + self.0.hist_2 * VAG_LUT_DECODER[predict][1];

            self.0.hist_2 = self.0.hist_1;
            self.0.hist_1 = sample;

            i16::MAX.min((sample as i16).max(i16::MIN))
        });

        Some(samples)
    }
}
