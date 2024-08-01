use std::fmt;
use std::ops;
use std::path::PathBuf;

use fundsp::prelude::*;
use itertools::Itertools;
use rustfft::{FftPlanner, num_complex::Complex};

pub type Channel = Vec<Sample>;
pub type Sample = f32;
pub type FFTChannel = Vec<FFTSample>;
pub type FFTSample = Complex<Sample>;

// the fundamental structure of an audio clip in this project
#[derive(Clone)]
pub struct AudioClip {
    pub channels: Vec<Channel>,
    pub file_name: String,
    pub sample_rate: f64,
    pub max_amplitude: f32,
    pub num_channels: usize,
    pub num_samples: usize,
}

pub struct FFTResult {
    pub channels: Vec<FFTChannel>,
    pub frequency_resolution: f64,
}

impl AudioClip {
    pub fn new(source: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let wave = Wave::load(source)?;
        let sample_rate = wave.sample_rate();
        let duration = wave.duration();
        let max_amplitude = wave.amplitude();
        let num_channels = wave.channels();
        let num_samples: usize = (duration * sample_rate) as usize;
        let mut channels: Vec<Channel> = Vec::new();

        for channel_idx in 0..num_channels {
            let mut channel = Channel::new();
            for sample_idx in 0..num_samples {
                channel.push(wave.at(channel_idx, sample_idx));
            }
            channels.push(channel);
        }

        Ok(AudioClip {
            channels,
            file_name: source.to_str().unwrap().to_string(),
            sample_rate,
            max_amplitude,
            num_channels,
            num_samples,
        })
    }

    pub fn fft(&self) -> FFTResult {
        let mut planner = FftPlanner::<Sample>::new();
        let fft = planner.plan_fft_forward(self.num_samples);
        let mut fft_final: Vec<FFTChannel> = Vec::new();

        // perform the FFT on each channel
        for channel in self.channels.iter() {
            let mut buffer = channel
                .iter()
                .map(|f| Complex::new(*f, 0.0))
                .collect_vec();
            fft.process(&mut buffer);
            fft_final.push(buffer);
        }

        FFTResult {
            channels: fft_final,
            frequency_resolution: self.sample_rate / self.num_samples as f64,
        }
    }

    // zero pads the audio clip; this is useful for comparison of two audio clips
    pub fn zero_pad(&self, num_samples: usize) -> Result<Self, Box<dyn std::error::Error>> {
        assert!(num_samples >= self.num_samples);

        let mut clip = self.clone();
        for channel in clip.channels.iter_mut() {
            channel.resize(num_samples, 0.0);
        }
        clip.num_samples = num_samples;
        Ok(clip)
    }

}

impl FFTResult {
    #[allow(dead_code)]
    pub fn dump(&self) -> Result<(), Box<dyn std::error::Error>> {
        let output = PathBuf::from("python/fft.csv");
        let mut wtr = csv::Writer::from_path(output)?;
        wtr.write_record(&["channel", "frequency", "norm"])?;
        for (channel_idx, channel) in self.channels.iter().enumerate() {
            for (i, sample) in channel.iter().enumerate() {
                let frequency = self.frequency_resolution * i as f64;
                wtr.write_record(&[channel_idx.to_string(), frequency.to_string(), sample.norm().to_string()])?;
            }
        }

        Ok(())
    }
}

impl fmt::Debug for AudioClip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioClip")
            .field("file_name", &self.file_name)
            .field("sample_rate", &self.sample_rate)
            .field("max_amplitude", &self.max_amplitude)
            .field("num_channels", &self.num_channels)
            .field("num_samples", &self.num_samples)
            .finish()
    }
}

impl fmt::Debug for FFTResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FFTResult")
            .field("frequency_resolution", &self.frequency_resolution)
            .finish()
    }
}


#[cfg(test)]
mod tests {
    use std::path;

    use super::*;

    #[test]
    fn test_create_audio_clip() {
        let source = path::PathBuf::from("test_sources/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");

        assert_ne!(clip.num_channels, 0);
        assert_ne!(clip.num_samples, 0);
        assert_ne!(clip.channels.len(), 0);

        assert_eq!(clip.channels.len(), clip.num_channels);
        assert_eq!(clip.channels[0].len(), clip.num_samples);
    }
}