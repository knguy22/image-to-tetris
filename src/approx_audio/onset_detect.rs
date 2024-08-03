use itertools::Itertools;

use super::audio_clip::{AudioClip, Sample};
use super::fft::FFTResult;

pub type  Onsets = Vec<Onset>;
pub struct Onset {
    pub index: usize,
    pub is_onset: bool,
}

impl AudioClip {
    // gives a vector of sample indices that are onsets
    // this currently uses spectrum onset detection
    pub fn detect_onsets(&self) -> Onsets {
        // perform short time fourier transform
        let window_size = 2048;
        let hop_size = 2048 / 4;
        let stft = self.stft(Some(window_size), Some(hop_size));

        // do feature processing
        let stft = self.apply_gamma_log(&stft, 100.0);

        // take the derivative
        let diffs = self.find_diffs(&stft);

        // perform onset detection using the derivative
        // onsets will typically have higher derivative values
        let mut onsets = Vec::new();
        let index_iter = (0..self.num_samples).step_by(hop_size);
        let avg_diff = diffs
            .iter()
            .sum::<f32>()
            / diffs.len() as f32;
        for (diff, index) in diffs.iter().zip(index_iter) {
            onsets.push(Onset {
                index,
                is_onset: *diff > avg_diff
            })
        }

        onsets
    }

    // effects: increases prominence of higher frequencies
    fn apply_gamma_log(&self, stft: &Vec<FFTResult>, gamma: Sample) -> Vec<Vec<Sample>> {
        stft
            .iter()
            .map(|fft_result| {
                fft_result
                    .samples
                    .iter()
                    .map(|sample| (1.0 + gamma * sample.norm()).ln())
                    .collect_vec()
            })
            .collect_vec()
    }

    // finds the diffs of an stft
    // this is equivalent to finding the derivative of the stft
    // negative derivatives are ignored since we don't care about offsets
    fn find_diffs(&self, stft: &Vec<Vec<Sample>>) -> Vec<Sample> {
        let mut diffs = stft
            .iter()
            .tuple_windows()
            .map(|(a, b)| {
                b
                    .iter()
                    .zip(a.iter())
                    .map(|(b, a)| if b - a >= 0.0 { b - a } else { 0.0 })
                    .reduce(|a, b| a + b)
                    .unwrap()
            })
            .collect_vec();
        diffs.insert(0, 0.0);
        diffs
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path;

    #[test]
    fn test_onsets() {
        let clip = AudioClip::new(&path::PathBuf::from("test_audio_clips/comboTones.mp3")).unwrap();
        let onsets = clip.detect_onsets();
        let onset_count = onsets
            .iter()
            .filter(|&b| b.is_onset)
            .count();

        assert!(onset_count > 0);
        assert!(onset_count < clip.num_samples);
    }
}