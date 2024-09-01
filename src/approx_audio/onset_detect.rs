use itertools::Itertools;

use super::audio_clip::{AudioClip, Sample};
use super::fft::{get_norms, FFTNorms, STFTNorms};
use super::windowing::{hanning_window, rectangle_window};

use anyhow::Result;

/// a vector of sample indices that contain properly detected onsets
pub type Onsets = Vec<usize>;

/// the difference between two fft timestamps, expressed as a single sample
type FFTDiff = Sample;

/// multiple `FFTDiffs` over different timestamps
type STFTDiffs = Vec<FFTDiff>;

impl AudioClip {
    pub fn split_by_onsets(&self) -> Vec<AudioClip> {
        let mut onsets = self.detect_onsets();

        // we need to include the beginning and the end in the onsets to include the whole clip
        if onsets[0] != 0 {
            onsets.insert(0, 0);
        }
        if onsets[onsets.len() - 1] != self.num_samples {
            onsets.push(self.num_samples);
        }

        onsets
            .iter()
            .tuple_windows()
            .map(|(&start, &end)| {
                // don't modify the original input through windowing because we're only splitting
                self.window(start, end, rectangle_window)
            })
            .collect_vec()
    }

    // gives a vector of sample indices that are onsets
    // this currently uses spectrum onset detection
    #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn detect_onsets(&self) -> Onsets {
        // perform short time fourier transform
        let window_size = 2048;
        let hop_size = window_size / 4;
        let stft = self.stft(window_size, hop_size, hanning_window);

        // transform the stft from complex into norms
        let stft = get_norms(&stft);

        // do feature processing
        let stft = apply_gamma_log(&stft, 100.0);

        // take the derivative
        let mut diffs = find_diffs(&stft);

        // use local averages to find extraordinary diffs
        let window_sec = 0.1;
        let window_size = (window_sec * self.sample_rate as Sample / hop_size as Sample).ceil() as usize;
        let local_avg_diffs = find_local_avgs(&diffs, window_size);
        for (diff, local_avg_diff) in diffs.iter_mut().zip(local_avg_diffs.iter()) {
            *diff = Sample::max(*diff - local_avg_diff, 0.0);
        }

        // normalize the diffs so we can use them for onset detection
        let diffs = normalize_diffs(&diffs);

        // perform onset detection using the derivative
        // onsets will typically have non-zero derivative values
        let mut onsets = Vec::new();
        let index_iter = (0..self.num_samples).step_by(hop_size);
        let mut last_onset = None;
        for (&diff, index) in diffs.iter().zip_eq(index_iter) {
            // only push onset once the diff is non-zero to a certain degree
            if last_onset.is_none() && diff > 0.2 {
                onsets.push(index);
                last_onset = Some(index);
            }
            else if index - last_onset.unwrap_or(0) > (0.2 * self.sample_rate) as usize {
                last_onset = None;
            }
        }

        onsets
    }
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
                // ignore a negative derivative
                total_diff += Sample::max(next_norm - curr_norm, 0.0);
            }
        }

        total_diff
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

#[allow(clippy::cast_precision_loss)]
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

#[allow(unused)]
fn dump_diffs(diffs: &STFTDiffs, output: &str) -> Result<()> {
    let mut wtr = csv::Writer::from_path(output)?;
    wtr.write_record(["diff", "magnitude"])?;
    for (i, diff) in diffs.iter().enumerate() {
        wtr.write_record(&[i.to_string(), diff.to_string()])?;
    }

    Ok(())
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
            .count();

        assert!(onset_count > 0);
        assert!(onset_count < clip.num_samples);
    }

    #[test]
    fn test_split_by_onsets() {
        let clip = AudioClip::new(&path::Path::new("test_audio_clips/comboTones.mp3")).unwrap();
        let onsets = clip.detect_onsets();
        let clips = clip.split_by_onsets();
        let total_num_samples: usize = clips
            .iter()
            .map(|clip| clip.num_samples)
            .sum();

        // should be 1 more than the number of onsets because clips are split by onsets
        assert_eq!(clips.len() - 1, onsets.len());
        assert_eq!(total_num_samples, clip.num_samples);
    }
}