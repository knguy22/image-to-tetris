use itertools::Itertools;

use super::audio_clip::Sample;
use super::fft::{FFTResult, FFTSample};

impl FFTResult {
    pub fn pitch_shift(&self, multiplier: Sample) -> Self {
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

    /// yields a tuple of (frequency, Vec[sample] = bin containing complex samples for each channel)
    /// yields up to the Nyquist frequency
    fn iter_zip_bins(&self) -> impl Iterator<Item = (Sample, Vec<FFTSample>)> + '_ {
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

        ifft_clip.write(Some(output)).unwrap();

    }
}