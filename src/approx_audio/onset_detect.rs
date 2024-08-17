use itertools::Itertools;

use super::audio_clip::{AudioClip, Sample};
use super::fft::FFTResult;
use super::windowing::rectangle_window;

pub type  Onsets = Vec<Onset>;
#[derive(Debug)]
pub struct Onset {
    pub index: usize,
    pub is_onset: bool,
}

/// a channel of norms; usually converted from a channel of complex samples
type FFTChannelNorm = Vec<Sample>;

/// multiple `FFTChannelNorms` over different channels
/// 
/// indexed by channel,sample
type FFTNorms = Vec<FFTChannelNorm>;

/// multiple `FFTNorms` over different timestamps
/// 
/// indexed by timestamp,channel,sample
type STFTNorms = Vec<FFTNorms>;

/// the difference between two fft timestamps, expressed as a single sample
type FFTDiff = Sample;

/// multiple `FFTDiffs` over different timestamps
type STFTDiffs = Vec<FFTDiff>;

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
        let mut diffs = normalize_diffs(&diffs);

        // use local averages to find extraordinary diffs
        let window_size = (self.sample_rate as Sample / hop_size as Sample).ceil() as usize;
        let local_avg_diffs = find_local_avgs(&diffs, window_size);
        for (diff, local_avg_diff) in diffs.iter_mut().zip(local_avg_diffs.iter()) {
            *diff = Sample::max(*diff - local_avg_diff, 0.0);
        }

        // perform onset detection using the derivative
        // onsets will typically have higher derivative values
        let mut onsets = Vec::new();
        let index_iter = (0..self.num_samples).step_by(hop_size);
        for (&diff, index) in diffs.iter().zip(index_iter) {
            onsets.push(Onset {
                index,
                is_onset: diff > 0.0
            })
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

/// this is mostly equivalent to finding the derivative of the stft
/// derivative = rate of gain of energy over time
/// onsets have higher derivative values
fn find_diffs(stft: &STFTNorms) -> STFTDiffs {
    fn delta(curr: &FFTNorms, next: &FFTNorms) -> FFTDiff {
        // each norm contains a vector of channel norms
        // we will compare channel to channel, norm to norm
        let mut total_diff = 0.0;
        for (curr_channel, next_channel) in curr.iter().zip_eq(next.iter()) {
            for (curr_norm, next_norm) in curr_channel.iter().zip_eq(next_channel.iter()) {
                total_diff += next_norm - curr_norm;
            }
        }

        // ignore a negative derivative
        FFTDiff::max(total_diff, 0.0)
    }

    let mut diffs = stft
        .iter()
        .tuple_windows()
        .map(|(curr, next)| delta(curr, next))
        .collect_vec();

    // the first differential is 0 since there is no preceding sample
    diffs.insert(0, 0.0);

    diffs
}

// reduce the scale so diffs are at most 1.0
fn normalize_diffs(diffs: &STFTDiffs) -> STFTDiffs {
    let max = diffs
        .iter()
        .reduce(|a, b| if a > b { a } else { b })
        .unwrap();
    assert!(!max.is_nan());
    assert!(*max > 0.0);

    diffs
        .iter()
        .map(|diff| diff / max)
        .collect_vec()
}

fn find_local_avgs(diffs: &STFTDiffs, window_size: usize) -> STFTDiffs {
    let mut local_diffs = Vec::new();

    // sliding window terms
    let mut r = 0;
    let mut window_sum: Sample = 0.0;

    for l in 0..diffs.len() {
        while r < diffs.len() && r - l < window_size {
            window_sum += diffs[r];
            r += 1;
        }
        assert!(r - l > 0, "r - l should never <= 0");

        local_diffs.push(window_sum / (r - l) as Sample);
        window_sum -= diffs[l];
    }

    assert!(local_diffs.len() == diffs.len(), "local_diffs.len() should == diffs.len()");

    local_diffs
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