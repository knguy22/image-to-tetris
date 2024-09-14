use itertools::Itertools;

use crate::approx_audio::audio_clip::Channel;
use super::audio_clip::{AudioClip, Sample};
use super::fft::{get_norms, FFTNorms, FFTResult, FFTSample, STFTNorms};
use super::windowing::{hanning_window, rectangle_window};

use anyhow::Result;

/// a vector of sample indices that contain properly detected onsets
pub type Onsets = Vec<usize>;

impl AudioClip {
    pub fn split_by_onsets(&self) -> Vec<AudioClip> {
        let mut onsets = self.detect_onsets_spectrum();

        // we need to include the beginning and the end in the onsets to include the whole clip
        if onsets[0] > 0 {
            onsets.insert(0, 0);
        }
        assert!(onsets.first().unwrap() == &0);

        if onsets[onsets.len() - 1] < self.num_samples {
            onsets.push(self.num_samples);
        }
        assert!(onsets.last().unwrap() == &self.num_samples);

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
    // method sourced from: https://www.audiolabs-erlangen.de/resources/MIR/FMP/C6/C6S1_NoveltySpectral.html
    #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss, clippy::cast_possible_truncation, unused)]
    fn detect_onsets_spectrum(&self) -> Onsets {
        // perform short time fourier transform
        let window_size = 2048;
        let hop_size = window_size / 4;
        let stft = self.stft(window_size, hop_size, hanning_window);

        // transform the stft from complex into norms
        let stft = get_norms(&stft);

        // do feature processing
        let stft = apply_gamma_log(&stft, 100.0);

        // take the derivative
        let diffs = find_diffs(&stft);
        let diffs = filter_non_negs_diffs(&diffs);
        let mut collapsed_diffs = collapse_diffs(&diffs);

        // use local averages to find extraordinary diffs
        let window_sec = 0.1;
        let window_size = (window_sec * self.sample_rate as Sample / hop_size as Sample).ceil() as usize;
        let local_avg_diffs = find_local_avgs(&collapsed_diffs, window_size);
        for (diff, local_avg_diff) in collapsed_diffs.iter_mut().zip(local_avg_diffs.iter()) {
            *diff = Sample::max(*diff - local_avg_diff, 0.0);
        }

        // normalize the diffs so we can use them for onset detection
        let diffs = normalize_diffs(&collapsed_diffs);

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

    /// method sourced from here: https://www.audiolabs-erlangen.de/resources/MIR/FMP/C6/C6S1_NoveltyPhase.html
    #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss, clippy::cast_possible_truncation, unused)]
    pub fn detect_onsets_phase(&self) -> Onsets {
        // perform short time fourier transform
        let window_size = 2048;
        let hop_size = window_size / 4;
        let stft = self.stft(window_size, hop_size, hanning_window);

        // obtain the phase
        let phase = find_phase_stft(&stft);

        // take two derivatives
        let diff_1 = principal_argument(&find_diffs(&phase));
        let diff_2 = principal_argument(&find_diffs(&diff_1));
        let mut collapsed_diffs = collapse_diffs(&filter_non_negs_diffs(&diff_2));

        // use local averages to find extraordinary diffs
        let window_sec = 0.1;
        let window_size = (window_sec * self.sample_rate as Sample / hop_size as Sample).ceil() as usize;
        let local_avg_diffs = find_local_avgs(&collapsed_diffs, window_size);
        for (diff, local_avg_diff) in collapsed_diffs.iter_mut().zip(local_avg_diffs.iter()) {
            *diff = Sample::max(*diff - local_avg_diff, 0.0);
        }

        // normalize the diffs so we can use them for onset detection
        let diffs = normalize_diffs(&collapsed_diffs);

        // perform onset detection using the derivative
        // onsets will typically have non-zero derivative values
        let mut onsets = Vec::new();
        let index_iter = (0..self.num_samples).step_by(hop_size);
        let mut last_onset = None;
        for (&diff, index) in diffs.iter().zip_eq(index_iter) {
            // only push onset once the diff is non-zero to a certain degree
            if last_onset.is_none() && diff > 0.175 {
                onsets.push(index);
                last_onset = Some(index);
            }
            else if index - last_onset.unwrap_or(0) > (0.1 * self.sample_rate) as usize {
                last_onset = None;
            }
        }

        onsets
    }
}

fn find_phase_stft(stft: &[FFTResult]) -> STFTNorms {
    fn find_phase_channel(channel: &[FFTSample]) -> Channel {
        channel.iter().map(|&sample| sample.to_polar().1 / (2.0 * std::f32::consts::PI)).collect_vec()
    }

    fn find_phase_fft(fft_result: &FFTResult) -> FFTNorms {
        fft_result
            .channels
            .iter()
            .map(|channel| find_phase_channel(channel))
            .collect_vec()
    }

    stft
        .iter()
        .map(find_phase_fft)
        .collect_vec()
}

/// this is mostly equivalent to finding the derivative of the stft
fn find_diffs(stft: &STFTNorms) -> STFTNorms {
    fn delta_channel(curr: &Channel, next: &Channel) -> Channel {
        curr.iter().zip_eq(next.iter())
            .map(|(&curr, &next)| next - curr)
            .collect_vec()
    }

    fn delta_fft_norm(curr: &FFTNorms, next: &FFTNorms) -> Vec<Channel> {
        // each norm contains a vector of channel norms
        // we will compare channel to channel, norm to norm
        curr.iter().zip_eq(next.iter())
            .map(|(curr_channel, next_channel)| delta_channel(curr_channel, next_channel))
            .collect_vec()
    }

    let mut diffs = stft
        .iter()
        .tuple_windows()
        .map(|(curr, next)| delta_fft_norm(curr, next))
        .collect_vec();

    // the first differential is 0 since there is no preceding sample
    let num_channels = stft[0].len();
    let num_bins = stft[0][0].len();
    diffs.insert(0, vec![vec![0.0; num_bins]; num_channels]);

    diffs
}

fn filter_non_negs_diffs(stft: &STFTNorms) -> STFTNorms {
    stft
        .iter()
        .map(|fft_norm| fft_norm
            .iter()
            .map(|channel| channel
                .iter()
                .map(|&sample| Sample::max(sample, 0.0))
                .collect_vec())
            .collect_vec())
        .collect_vec()
}

/// collapses the diffs into a single number for each time step
fn collapse_diffs(stft: &STFTNorms) -> Vec<Sample> {
    stft
        .iter()
        .map(|fft_norm| fft_norm
            .iter()
            .map(|channel| channel.iter().sum::<Sample>())
            .sum::<Sample>())
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

// reduce the scale so diffs are at most 1.0
fn normalize_diffs(diffs: &[Sample]) -> Vec<Sample> {
    let max = diffs
        .iter()
        .reduce(|a, b| if a > b { a } else { b })
        .unwrap();
    assert!(max.is_finite());
    assert!(*max > 0.0);

    diffs
        .iter()
        .map(|diff| diff / max)
        .collect_vec()
}

/// equivalent to np.mod(`stft` + 0.5, 1) - 0.5
fn principal_argument(stft: &STFTNorms) -> STFTNorms {
    stft.iter()
        .map(|fft_norm| fft_norm
            .iter()
            .map(|channel| channel
                .iter()
                .map(|&sample| Sample::rem_euclid(sample + 0.5, 1.0) - 0.5)
                .collect_vec())
            .collect_vec())
        .collect_vec()
}

#[allow(clippy::cast_precision_loss)]
fn find_local_avgs(samples: &[Sample], window_size: usize) -> Vec<Sample> {
    let mut local_diffs = Vec::new();

    // sliding window terms
    let mut r = 0;
    let mut window_sum: Sample = 0.0;

    for l in 0..samples.len() {
        while r < samples.len() && r - l < window_size {
            window_sum += samples[r];
            r += 1;
        }
        assert!(r - l > 0, "r - l should never <= 0");

        local_diffs.push(window_sum / window_size as Sample);
        window_sum -= samples[l];
    }

    assert!(local_diffs.len() == samples.len(), "local_diffs.len() should == diffs.len()");

    local_diffs
}

#[allow(unused)]
fn dump_diffs(diffs: &[Sample], output: &str) -> Result<()> {
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
    fn test_diffs() {
        let clip = AudioClip::new(&path::Path::new("test_audio_clips/comboTones.mp3")).unwrap();

        let window_size = 2048;
        let hop_size = window_size / 4;
        let stft = clip.stft(window_size, hop_size, hanning_window);
        let norms = get_norms(&stft);
        let diffs = find_diffs(&norms);

        assert!(diffs.iter().all(|diff| diff.len() == diffs[0].len()));
    }

    #[test]
    fn test_onsets() {
        let clip = AudioClip::new(&path::Path::new("test_audio_clips/comboTones.mp3")).unwrap();
        let onsets = clip.detect_onsets_spectrum();
        let onset_count = onsets
            .iter()
            .count();

        assert!(onset_count > 0);
        assert!(onset_count < clip.num_samples);
    }

    #[test]
    #[ignore]
    fn test_split_by_onsets() {
        let clip = AudioClip::new(&path::Path::new("test_audio_clips/comboTones.mp3")).unwrap();
        let onsets = clip.detect_onsets_phase();
        let clips = clip.split_by_onsets();
        let total_num_samples: usize = clips
            .iter()
            .map(|clip| clip.num_samples)
            .sum();

        // should be 1 more than the number of onsets because clips are split by onsets
        assert_eq!(clips.len() - 1, onsets.len(), "number of clips should be 1 less than number of onsets");
        assert_eq!(total_num_samples, clip.num_samples, "total number of samples should equal original number of samples");
    }
}