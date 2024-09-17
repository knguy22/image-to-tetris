use crate::utils::check_command_result;
use super::audio_clip::{AudioClip, Sample};
use super::fft::{FFTResult, FFTSample};

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use rustfft::num_complex::Complex;
use itertools::Itertools;

/// precomputed chromatic difference; 2.0^(1/12)
pub static CHROMATIC_MULTIPLIER: Sample = 1.059_463_1;

/// the frequency and magnitude of a bin
type FreqBin = (Sample, Vec<FFTSample>);

impl AudioClip {
    #[allow(unused)]
    pub fn pitch_shift(&self, multiplier: Sample) -> Result<Self> {
        let tmp_input = Path::new("tmp_input.wav");
        let tmp_output = Path::new("tmp_output.wav");

        // dump and resample the audio using pitch shifting
        self.write(Some(tmp_input))?;
        let resample_command = Command::new("ffmpeg")
            .arg("-i")
            .arg(tmp_input)
            .arg("-filter:a")
            .arg(format!("asetrate={}*{},aresample={}", self.sample_rate, multiplier, self.sample_rate))
            .arg(tmp_output)
            .output()?;
        check_command_result(&resample_command)?;
        let res = Self::new(tmp_output)?;

        // cleanup
        fs::remove_file(tmp_input)?;
        fs::remove_file(tmp_output)?;

        Ok(res)
    }
}

impl FFTResult {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, unused)]
    pub fn pitch_shift(&self, multiplier: Sample) -> Self {
        assert!(multiplier >= 0.0);

        let mut result = FFTResult::empty(self.sample_rate, self.num_samples, self.channels.len());

        // adjust each bin to a new frequency and bin
        for (freq, bin) in self.iter_zip_bins() {
            let new_freq = freq * multiplier;
            let new_bin_index = (new_freq / self.frequency_resolution as Sample) as usize;

            // ignore frequencies that are out of range
            if new_bin_index >= self.num_samples {
                continue
            }

            for (sample, channel) in bin.iter().zip_eq(result.channels.iter_mut()) {
                channel[new_bin_index] += sample;
            }
        }

        result
    }

    pub fn most_significant_frequency(&self) -> Sample {
        // we take the bins with the higher norms/energy level
        fn compare_bins(a: &FreqBin, b: &FreqBin) -> FreqBin {
            let a_norms = a.1.iter().map(|s| s.norm()).fold(0.0, |a, b| a + b);
            let b_norms = b.1.iter().map(|s| s.norm()).fold(0.0, |a, b| a + b);

            if a_norms > b_norms {
                a.clone()
            } else {
                b.clone()
            }
        }

        let most_significant_freq_bin = self.iter_zip_bins()
            .fold((-1.0, vec![Complex::new(0.0, 0.0); self.channels.len()]), 
                |a, b| compare_bins(&a, &b)
            );

        most_significant_freq_bin.0
    }

    /// yields a tuple of (frequency, Vec(sample) = bin containing complex samples for each channel)
    /// yields up to the Nyquist frequency
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn iter_zip_bins(&self) -> impl Iterator<Item = FreqBin> + '_ {
        let nyquist = self.nyquist_frequency() as Sample;

        (0..self.num_samples).map(|i| {
            let freq = self.frequency_resolution as Sample * i as Sample;
            let bin = self.channels.iter().map(|channel| channel[i]).collect_vec();
            (freq, bin)
        })
        .take_while(move |(freq, _)| *freq <= nyquist)
    }

    fn nyquist_frequency(&self) -> f64 {
        self.sample_rate / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approx_audio::AudioClip;
    use std::path::Path;
    use rust_lapper::Interval;

    #[test]
    fn test_pitch_shift() {
        let source = Path::new("test_audio_clips/a6.mp3");
        let output = Path::new("test_pitch_shifted_clip.wav");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let multiplier = 0.5;

        let fft = clip.fft();
        let pitch_shifted = fft.pitch_shift(multiplier);
        let ifft_clip = pitch_shifted.ifft_to_audio_clip();

        assert!((clip.duration - ifft_clip.duration).abs() < 0.001);
        assert!(clip.sample_rate == ifft_clip.sample_rate);

        // there is no need to check max amplitude for exact correctness because the channels have been merged with the ifft, which means
        // amplitudes are averaged out
        assert!(ifft_clip.max_amplitude <= clip.max_amplitude);

        // make sure the most important frequencies are the same
        let source_freq = clip.fft().most_significant_frequency();
        let expected_shifted_freq = source_freq * multiplier;
        let expected_interval = Interval { start: (expected_shifted_freq / CHROMATIC_MULTIPLIER) as usize, stop: (expected_shifted_freq * CHROMATIC_MULTIPLIER) as usize + 1, val: 0};
        let shifted_freq = pitch_shifted.most_significant_frequency() as usize;
        assert!(expected_interval.start <= shifted_freq);
        assert!(expected_interval.stop >= shifted_freq);

        ifft_clip.write(Some(output)).unwrap();
    }
}