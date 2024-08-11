use itertools::Itertools;

use super::audio_clip::{AudioClip, Sample, Channel};
use super::fft::FFTResult;
use super::windowing::rectangle_window;

pub type  Onsets = Vec<Onset>;
#[derive(Debug)]
pub struct Onset {
    pub index: usize,
    pub is_onset: bool,
}

// each fft is computed per channel
// stft is a combination of ffts
// thus, indexing will generally go FFT clip -> channel -> sample
type FFTDiffs = Channel;
type FFTNorms = Vec<Channel>;
type STFTDiffs = Vec<FFTDiffs>;
type STFTNorms = Vec<FFTNorms>;

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
                self.window(start, end, rectangle_window)
            })
            .collect_vec()
    }

    // gives a vector of sample indices that are onsets
    // this currently uses spectrum onset detection
    #[allow(clippy::cast_precision_loss)]
    fn detect_onsets(&self) -> Onsets {
        // perform short time fourier transform
        let window_size = 2048;
        let hop_size = window_size / 4;
        let stft = self.stft(window_size, hop_size);

        // transform the stft from complex into norms
        let stft = get_norms(&stft);

        // do feature processing
        let stft = apply_gamma_log(&stft, 100.0);

        // take the derivative
        let diffs = find_diffs(&stft);
        let diffs = normalize_diffs(&diffs);

        // find the average diff for each channel
        // also, recall that diffs is indexed by diffs[FFT clip][channel][sample]
        let mut avg_diff: Vec<Sample> = Vec::new();
        for i in 0..self.num_channels {
            avg_diff.push(diffs.iter().map(|fft_diff| fft_diff[i]).sum::<Sample>() / diffs.len() as Sample);
        }

        // perform onset detection using the derivative
        // onsets will typically have higher derivative values
        let mut onsets = Vec::new();
        let index_iter = (0..self.num_samples).step_by(hop_size);
        for index in index_iter {
            // find an average diff
            let diffs = self.channels.iter().map(|channel| channel[index]).collect_vec();
            let mut zipped_diffs = diffs.iter().zip_eq(avg_diff.iter());

            onsets.push(Onset {
                index,
                is_onset: zipped_diffs.any(|(diff, avg)| diff > avg),
            });
        }

        onsets
    }
}

fn get_norms(stft: &[FFTResult]) -> STFTNorms {
    fn norms_fft_result(fft_result: &FFTResult) -> FFTNorms {
        fft_result
            .channels
            .iter()
            .map(|channel| channel.iter().map(|&sample| sample.norm()).collect_vec())
            .collect_vec()
    }

    stft
        .iter()
        .map(norms_fft_result)
        .collect_vec()
}

// effects: increases prominence of higher frequencies
fn apply_gamma_log(stft: &STFTNorms, gamma: Sample) -> STFTNorms {
    fn gamma_log_fft_norm(fft_norm: &FFTNorms, gamma: Sample) -> FFTNorms {
        fft_norm
            .iter()
            .map(|channel|
                channel.iter().map(|&sample| (1.0 + gamma * sample).ln()).collect_vec())
            .collect_vec()
    }

    stft
        .iter()
        .map(|fft_norm| gamma_log_fft_norm(fft_norm, gamma))
        .collect_vec()
}

// finds the diffs of an stft
// this is equivalent to finding the derivative of the stft
// negative derivatives are ignored since we don't care about offsets
fn find_diffs(stft: &STFTNorms) -> STFTDiffs {
    fn diff_fft_norm(fft_norm: &FFTNorms) -> FFTDiffs {
        let mut diffs = fft_norm
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

        // insert 0 at index 0 to maintain same length
        diffs.insert(0, 0.0);
        diffs
    }

    stft
        .iter()
        .map(diff_fft_norm)
        .collect_vec()
}

// reduce the scale so diffs are at most 1.0
fn normalize_diffs(diffs: &STFTDiffs) -> STFTDiffs {
    fn normalize_diff(diff: &FFTDiffs) -> FFTDiffs {
        let max = diff
            .iter()
            .reduce(|a, b| if a > b { a } else { b })
            .unwrap();
        assert!(!max.is_nan());

        if *max == 0.0 {
            return diff.clone();
        }
        diff
            .iter()
            .map(|diff| diff / max)
            .collect_vec()
    }

    diffs
        .iter()
        .map(normalize_diff)
        .collect_vec()
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path;

    #[test]
    fn test_onsets() {
        let clip = AudioClip::new(&path::Path::new("test_audio_clips/comboTones.mp3")).unwrap();
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
        let clip = AudioClip::new(&path::Path::new("test_audio_clips/comboTones.mp3")).unwrap();
        let onsets = clip.detect_onsets();
        let true_onsets = onsets.iter().filter(|o| o.is_onset).collect_vec();
        let clips = clip.split_by_onsets();

        // should be 1 more than the number of onsets because clips are split by onsets
        assert_eq!(clips.len() - 1, true_onsets.len());
    }
}