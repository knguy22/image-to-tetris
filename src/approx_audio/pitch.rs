use super::audio_clip::Sample;
use super::fft::{FFTResult, FFTSample};

use itertools::Itertools;
use rustfft::num_complex::Complex;
use rust_lapper::{Lapper, Interval};

/// the frequency and magnitude of a bin
type FreqBin = (Sample, Vec<FFTSample>);

#[derive(Debug)]
pub struct NoteTracker {
    lapper: Lapper<usize, usize>,
}

impl NoteTracker {
    pub fn new() -> Self {
        Self {
            lapper: Lapper::new(Vec::new()),
        }
    }

    pub fn add_note(&mut self, freq: Sample, val: usize) -> Option<()> {
        // only add notes that aren't already found
        if self.get_note(freq).is_some() {
            return None;
        }

        self.lapper.insert(Self::interval(freq, val));
        Some(())
    }
    
    pub fn get_note(&self, freq: Sample) -> Option<usize> {
        let freq_interval = Self::interval(freq, 0);
        let res = self.lapper.find(freq_interval.start, freq_interval.stop).collect_vec();
        assert!(res.len() <= 1, "found more than one note at frequency {}", freq);

        res.into_iter().next().map(|i| i.val)
    }

    /// creates an interval based on the frequency that only overlaps with notes in the same chromatic note
    pub fn interval(freq: Sample, val: usize) -> Interval<usize, usize> {
        // chromatic notes differ in frequency by a multiple of 2^(1/12)
        // to prevent two chromatic notes from overlapping in intervals, we take another square root of the multiplier
        // and subtract by a small constant to account for precision errors
        let coefficient: Sample = Sample::from(2.0).powf(1.0 / 12.0).powf(0.5) - 0.01;

        let start = (freq / coefficient) as usize;
        let stop = (freq * coefficient) as usize;

        Interval { start, stop, val }
    }
}

impl FFTResult {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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

    /// yields a tuple of (frequency, Vec[sample] = bin containing complex samples for each channel)
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
    use crate::approx_audio::AudioClip;
    use std::path::Path;
    use super::*;

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
        let expected_interval = NoteTracker::interval(expected_shifted_freq, 0);
        let shifted_freq = pitch_shifted.most_significant_frequency() as usize;
        assert!(expected_interval.start <= shifted_freq);
        assert!(expected_interval.stop >= shifted_freq);

        ifft_clip.write(Some(output)).unwrap();
    }

    #[test]
    fn test_note_tracker() {
        let mut note_tracker = NoteTracker::new();
        note_tracker.add_note(440.0, 0);

        assert!(note_tracker.get_note(440.0).is_some());
        assert!(note_tracker.get_note(440.0 * 1.3).is_none());
        assert!(note_tracker.get_note(440.0 / 1.3).is_none());
    }
}