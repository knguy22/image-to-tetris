use itertools::Itertools;

use super::audio_clip::{AudioClip, Sample};
use super::fft::FFTResult;

pub type  Onsets = Vec<Onset>;
#[derive(Debug)]
pub struct Onset {
    pub index: usize,
    pub is_onset: bool,
}

impl AudioClip {
    pub fn split_by_onsets(&self) -> Vec<AudioClip> {
        let onsets = self.detect_onsets();
        let mut true_onsets = onsets.iter().filter(|o| o.is_onset).collect_vec();

        // we need to include 0 and the end in the true onsets
        if true_onsets[0].index != 0 {
            true_onsets.insert(0, &Onset {
                index: 0,
                is_onset: true
            });
        }

        let end = Onset {
            index: self.num_samples,
            is_onset: true
        };
        if true_onsets[true_onsets.len() - 1].index != self.num_samples {
            true_onsets.push(&end);
        }

        true_onsets
            .iter()
            .tuple_windows()
            .map(|(a, b)| {
                let start = a.index;
                let end = b.index;
                self.window(start, end)
            })
            .collect_vec()
    }

    // gives a vector of sample indices that are onsets
    // this currently uses spectrum onset detection
    fn detect_onsets(&self) -> Onsets {
        // perform short time fourier transform
        let window_size = 2048;
        let hop_size = window_size / 4;
        let stft = self.stft(window_size, hop_size);

        // do feature processing
        let stft = self.apply_gamma_log(&stft, 100.0);

        // take the derivative
        let diffs = self.find_diffs(&stft);

        // perform onset detection using the derivative
        // onsets will typically have higher derivative values
        let mut onsets = Vec::new();
        let index_iter = (0..self.num_samples).step_by(window_size + hop_size);
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

    #[test]
    fn test_split_by_onsets() {
        let clip = AudioClip::new(&path::PathBuf::from("test_audio_clips/comboTones.mp3")).unwrap();
        let onsets = clip.detect_onsets();
        let true_onsets = onsets.iter().filter(|o| o.is_onset).collect_vec();
        let clips = clip.split_by_onsets();

        // should be 1 more than the number of onsets because clips are split by onsets
        assert_eq!(clips.len() - 1, true_onsets.len());
    }
}