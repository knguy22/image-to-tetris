use super::audio_clip::{AudioClip, Sample};
use std::fmt;
use std::path::PathBuf;
use itertools::Itertools;
use rustfft::{FftPlanner, num_complex::Complex};

pub type FFTChannel = Vec<FFTSample>;
pub type FFTSample = Complex<Sample>;

pub struct FFTResult {
    pub channels: Vec<FFTChannel>,
    pub frequency_resolution: f64,
}

impl AudioClip {
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

impl fmt::Debug for FFTResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FFTResult")
            .field("frequency_resolution", &self.frequency_resolution)
            .finish()
    }
}

