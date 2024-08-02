use super::audio_clip::{AudioClip, Sample};
use std::fmt;
use std::path::PathBuf;
use itertools::Itertools;
use rustfft::{FftPlanner, num_complex::Complex};

pub type FFTSample = Complex<Sample>;

pub struct FFTResult {
    pub samples: Vec<FFTSample>,
    pub frequency_resolution: f64,
}

impl AudioClip {
    pub fn fft(&self) -> FFTResult {
        let mut planner = FftPlanner::<Sample>::new();
        let fft = planner.plan_fft_forward(self.num_samples);

        // average out all the samples across channels for each sample index
        // then turn the average into a complex number
        let mut average_samples = (0..self.num_samples)
            .map(|i| {
                self.channels
                    .iter()
                    .map(|c| c[i])
                    .reduce(|a, b| a + b)
                    .unwrap() / self.channels.len() as Sample
            })
            .map(|f| Complex::new(f, 0.0))
            .collect_vec();

        // perform the FFT using the averaged samples
        fft.process(&mut average_samples);

        FFTResult {
            samples: average_samples,
            frequency_resolution: self.sample_rate / self.num_samples as f64,
        }
    }
}

impl FFTResult {
    #[allow(dead_code)]
    pub fn dump(&self) -> Result<(), Box<dyn std::error::Error>> {
        let output = PathBuf::from("python/fft.csv");
        let mut wtr = csv::Writer::from_path(output)?;
        wtr.write_record(&["frequency", "norm"])?;
        for (i, sample) in self.samples.iter().enumerate() {
            let frequency = self.frequency_resolution * i as f64;
            wtr.write_record(&[frequency.to_string(), sample.norm().to_string()])?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path;

    #[test]
    fn test_fft() {
        let source = path::PathBuf::from("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        assert_ne!(fft.samples.len(), 0);
    }

    #[test]
    fn test_dump() {
        let source = path::PathBuf::from("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        fft.dump().unwrap();
    }
}