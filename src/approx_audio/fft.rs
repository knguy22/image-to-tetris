use super::audio_clip::{AudioClip, Sample};
use std::fmt;
use std::path::Path;
use itertools::Itertools;
use rustfft::{FftPlanner, num_complex::Complex};

pub type FFTSample = Complex<Sample>;

pub struct FFTResult {
    pub samples: Vec<FFTSample>,
    pub frequency_resolution: f64,
}

impl AudioClip {

    /// performs a short time fourier transform on the audio clip
    /// `window_size` is the number of samples in the window; defaults to 2048
    /// `hop_size` is the number of samples between each window; defaults to `window_size` // 4
    pub fn stft(&self, window_size: usize, hop_size: usize) -> Vec<FFTResult> {
        let mut stft_res = Vec::new();

        let mut curr_index = 0;
        while curr_index < self.num_samples {
            let window = self.window(curr_index, curr_index + window_size);
            stft_res.push(window.fft());
            curr_index += hop_size;
        }

        stft_res
    }

    #[allow(clippy::cast_precision_loss)]
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
    #[allow(clippy::cast_precision_loss)]
    pub fn ifft_to_audio_clip(&self) -> AudioClip {
        let samples = self.ifft();
        let sample_rate = self.samples.len() as f64 * self.frequency_resolution;
        let duration = self.samples.len() as f64 / sample_rate;

        let max_amplitude = samples
            .iter()
            .fold(0.0, |a, &b| if a > b { a } else { b });
        assert!(!max_amplitude.is_nan());

        AudioClip {
            channels: vec![samples],
            file_name: String::new(),
            duration,
            sample_rate,
            max_amplitude,
            num_channels: 1,
            num_samples: self.samples.len(),
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn ifft(&self) -> Vec<Sample> {
        let mut planner = FftPlanner::<Sample>::new();
        let fft = planner.plan_fft_inverse(self.samples.len());
        let mut ifft_samples = self.samples.clone();
        fft.process(&mut ifft_samples);

        // amplitudes across iffts are not standardize so we need to normalize them (with sample len)
        ifft_samples.iter().map(|s| s.norm() / self.samples.len() as Sample).collect()
    }

    #[allow(clippy::cast_precision_loss, dead_code)]
    pub fn dump(&self, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut wtr = csv::Writer::from_path(output)?;
        wtr.write_record(["frequency", "norm"])?;
        for (i, sample) in self.samples.iter().enumerate() {
            let frequency = self.frequency_resolution * i as f64;
            wtr.write_record(&[frequency.to_string(), sample.norm().to_string()])?;
        }

        Ok(())
    }
}

#[allow(clippy::missing_fields_in_debug)]
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
        let source = path::Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        assert_ne!(fft.samples.len(), 0);
        assert_eq!(fft.samples.len(), clip.num_samples);
    }

    #[test]
    fn test_fft_monotone() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let amplitude = 0.5;

        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude, 1);
        let fft = clip.fft();
        assert_eq!(fft.samples.len(), clip.num_samples);
    }

    #[test]
    fn test_stft() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let amplitude = 0.5;

        let window = 1024;
        let hop = window / 4;
        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude, 1);
        let stft = clip.stft(window, hop);

        assert_eq!(stft.len(), clip.num_samples / hop + 1);
    }

    #[test]
    fn test_stft_2() {
        let sample_rate = 24100.0;
        let duration = 5.6;
        let amplitude = 0.6;

        let window = 2048;
        let hop = window / 4;
        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude, 1);
        let stft = clip.stft(window, hop);

        assert_eq!(stft.len(), clip.num_samples / hop + 1);
    }

    #[test]
    fn test_dump() {
        let source = path::Path::new("test_audio_clips/a6.mp3");
        let output = path::Path::new("test_fft.csv");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        fft.dump(output).unwrap();

        // cleanup
        std::fs::remove_file(&output).unwrap();
    }

    #[test]
    fn test_ifft() {
        let source = path::Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        let ifft = fft.ifft();
        assert_eq!(ifft.len(), clip.num_samples);
    }

    #[test]
    fn test_ifft_clip() {
        let source = path::Path::new("test_audio_clips/a6.mp3");
        let output = path::Path::new("test_ifft_clip.wav");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        let ifft_clip = fft.ifft_to_audio_clip();

        assert!((clip.duration - ifft_clip.duration).abs() < 0.001);
        assert!(clip.sample_rate == ifft_clip.sample_rate);

        // there is no need to check max amplitude for exact correctness because the channels have been merged with the ifft, which means
        // amplitudes are averaged out
        assert!(ifft_clip.max_amplitude <= clip.max_amplitude);

        ifft_clip.write(Some(output)).unwrap();
    }
}