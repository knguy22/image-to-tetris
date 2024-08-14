use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::Result;
use fundsp::prelude::*;
use hound::{WavWriter, WavSpec, SampleFormat};
use thiserror::Error;

use super::windowing::rectangle_window;

/// not limited to direct samples but also coefficients applied onto samples
pub type Sample = f32; 
pub type Channel = Vec<Sample>;

// the fundamental structure of an audio clip in this project
#[derive(Clone)]
pub struct AudioClip {
    pub channels: Vec<Channel>,
    pub file_name: String,
    pub duration: f64,
    pub sample_rate: f64,
    pub max_amplitude: Sample,
    pub num_channels: usize,
    pub num_samples: usize,
}

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("Output not wav: {message}")]
    OutputNotWavError{message: String},
}

impl AudioClip {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn new(source: &Path) -> Result<Self> {
        let wave = Wave::load(source)?;
        let sample_rate = wave.sample_rate();
        let duration = wave.duration();
        let max_amplitude = Sample::from(wave.amplitude());
        let num_channels = wave.channels();
        let num_samples: usize = (duration * sample_rate) as usize;
        let mut channels: Vec<Channel> = Vec::new();

        for channel_idx in 0..num_channels {
            let mut channel = Channel::new();
            for sample_idx in 0..num_samples {
                channel.push(Sample::from(wave.at(channel_idx, sample_idx)));
            }
            channels.push(channel);
        }

        Ok(AudioClip {
            channels,
            file_name: source.to_str().unwrap().to_string(),
            duration,
            sample_rate,
            max_amplitude,
            num_channels,
            num_samples,
        })
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, dead_code)]
    pub fn new_monoamplitude(sample_rate: f64, num_samples: usize, amplitude: Sample, num_channels: usize) -> Self {
        let duration = num_samples as f64 / sample_rate;
        let channels: Vec<Channel> = vec![vec![amplitude; num_samples]; num_channels];

        AudioClip {
            channels,
            file_name: String::new(),
            duration,
            sample_rate,
            max_amplitude: amplitude,
            num_channels,
            num_samples,
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn write(&self, path: Option<&Path>) -> Result<()> {
        let path = path.unwrap_or(Path::new(&self.file_name));

        // output file must be wav
        if path.extension().unwrap() != "wav" {
            return Err(WriteError::OutputNotWavError{message: format!("Output not wav: {}", path.display())})?;
        }

        // create the output file and initialize the writer
        let spec = WavSpec {
            channels: u16::try_from(self.channels.len())?,
            sample_rate: self.sample_rate as u32,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };

        let output_file = fs::File::create(path)?;
        let writer = io::BufWriter::new(output_file);
        let mut wav_writer = WavWriter::new(writer, spec)?;

        // write each channel with interleaved samples
        assert!(self.channels.iter().all(|channel| channel.len() == self.num_samples));
        for i in 0..self.num_samples {
            for channel in &self.channels {
                wav_writer.write_sample(channel[i])?;
            }
        }

        wav_writer.finalize()?;
        Ok(())
    }

    // splits the audio clip into chunks the length of max_duration; if the last chunk is shorter than 
    // max_duration, it will still be included but will be smaller than max_duration
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn split_by_duration(&self, max_duration: f64) -> Vec<Self> {
        // split the original video into chunks; this will be useful for approximation later
        let mut chunks = Vec::new();
        let chunk_num_samples = (max_duration * self.sample_rate) as usize;
        for begin in (0..self.num_samples).step_by(chunk_num_samples) {
            let end = std::cmp::min(begin + chunk_num_samples, self.num_samples);
            let chunk = self.window(begin, end, rectangle_window);
            chunks.push(chunk);
        }
        chunks
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn diff(&self, other: &Self, multiplier: Sample) -> f64 {
        self.mse(other, multiplier)
    }

    // add new channels to the audio clip
    // uses the average of existing channels for new values
    #[allow(clippy::cast_precision_loss)]
    pub fn add_new_channels_mut(&mut self, num_channels: usize) {
        assert!(num_channels >= self.num_channels);
        if self.num_channels == num_channels {
            return
        }

        let new_channels = num_channels - self.num_channels;

        // add the new channels
        let mut channels = self.channels.clone();
        for _ in 0..new_channels {
            channels.push(vec![0.0; self.num_samples]);
        }

        // populate the existing channels with values
        for sample_idx in 0..self.num_samples {
            let mut sample_sum = 0.0;
            for channel_idx in 0..self.num_channels {
                sample_sum += self.channels[channel_idx][sample_idx];
            }

            for new_channel in self.channels.iter_mut().take(num_channels).skip(self.num_channels) {
                new_channel[sample_idx] = sample_sum / (self.num_channels as Sample);
            }
        }

        self.channels = channels;
        self.num_channels = num_channels;
    }

    /// add two audio clips up to the amount of samples `self` has
    /// extra samples beyond `self` will be ignored
    pub fn add_mut(&mut self, rhs: &Self, multiplier: Sample) {
        assert!(self.num_channels == rhs.num_channels);
        assert!((self.sample_rate - rhs.sample_rate).abs() < f64::EPSILON);

        let limit = std::cmp::min(self.num_samples, rhs.num_samples);
        for channel_idx in 0..self.num_channels {
            for sample_idx in 0..limit {
                self.channels[channel_idx][sample_idx] += rhs.channels[channel_idx].get(sample_idx).unwrap_or(&0.0) * multiplier;
            }
        }
    }

    #[allow(unused)]
    pub fn append_mut(&mut self, rhs: &Self) {
        assert!(self.num_channels == rhs.num_channels);
        assert!((self.sample_rate - rhs.sample_rate).abs() < f64::EPSILON);

        for channel_idx in 0..self.num_channels {
            self.channels[channel_idx].extend(&rhs.channels[channel_idx]);
        }
        self.num_samples += rhs.num_samples;
        self.max_amplitude = self.max_amplitude.max(rhs.max_amplitude);
        self.duration += rhs.duration;
    }

    pub fn scale_amplitude(&self, rhs: Sample) -> Self {
        let mut output = self.clone();
        for channel in &mut output.channels {
            for sample in channel {
                *sample *= rhs;
            }
        }
        output.max_amplitude *= rhs;
        output
    }

    #[allow(dead_code)]
    // zero pads the audio clip; this is useful for comparison of two audio clips
    pub fn zero_pad(&self, num_samples: usize) -> Self {
        assert!(num_samples >= self.num_samples);

        let mut clip = self.clone();
        for channel in &mut clip.channels {
            channel.resize(num_samples, 0.0);
        }
        clip.num_samples = num_samples;
        clip
    }

    #[allow(clippy::cast_precision_loss, dead_code)]
    pub fn dump(&self, output: &Path) -> Result<()> {
        let mut wtr = csv::Writer::from_path(output)?;
        wtr.write_record(["channel", "index" ,"magnitude"])?;
        for (i, sample) in self.channels.iter().enumerate() {
            for (j, magnitude) in sample.iter().enumerate() {
                wtr.write_record(&[i.to_string(), j.to_string(), magnitude.to_string()])?;
            }
        }

        Ok(())
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl fmt::Debug for AudioClip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioClip")
            .field("duration", &self.duration)
            .field("file_name", &self.file_name)
            .field("sample_rate", &self.sample_rate)
            .field("max_amplitude", &self.max_amplitude)
            .field("num_channels", &self.num_channels)
            .field("num_samples", &self.num_samples)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::path;

    use super::*;

    #[test]
    fn test_create_audio_clip() {
        let source = path::Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");

        assert_ne!(clip.num_channels, 0);
        assert_ne!(clip.num_samples, 0);
        assert_ne!(clip.channels.len(), 0);

        assert_eq!(clip.channels.len(), clip.num_channels);
        assert_eq!(clip.channels[0].len(), clip.num_samples);
    }

    #[test]
    fn test_monoamplitude() {
        let sample_rate = 44100.0;
        let num_samples = 44100;
        let amplitude = 0.5;

        let clip = AudioClip::new_monoamplitude(sample_rate, num_samples, amplitude, 1);

        assert!(clip.num_channels > 0);
        assert_eq!(clip.num_samples, num_samples);
        assert!(clip.channels.iter().all(|v| v.len() == clip.num_samples));
        assert!(clip.channels[0].iter().all(|v| *v == amplitude));
    }

    #[test]
    fn test_write_audio_clip() {
        let source = path::Path::new("test_audio_clips/a6.mp3");
        let output = path::Path::new("test_results/test.wav");

        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        clip.write(Some(&output)).expect("failed to write audio clip");

        assert!(output.exists());

        // cleanup
        fs::remove_file(output).expect("failed to remove test file");
    }

    #[test]
    fn test_split_clip() {
        let duration = 0.2;
        let source = path::Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip").split_by_duration(duration);

        assert_eq!(clip.len(), 15);
        assert!(clip.iter().all(|c| c.sample_rate == clip[0].sample_rate));

        // exclude last due to rounding errors
        for chunk in clip.iter().take(clip.len() - 1) {
            assert_eq!(chunk.duration, duration);
            assert_eq!(chunk.num_samples, chunk.channels[0].len());

            for channel in chunk.channels.iter() {
                assert_eq!(channel.len(), chunk.num_samples);
            }
        }

        let last = clip.last().unwrap();
        assert_ne!(last.duration, duration);
        assert_eq!(last.num_samples, last.channels[0].len());
    }

    #[test]
    fn test_zero_padding() {
        let num_samples = 1000000;
        let source = path::Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let output = clip.zero_pad(num_samples);

        assert_eq!(output.num_samples, num_samples);
        assert!(output.channels.iter().all(|channel| channel.len() == num_samples));
    }

    #[test]
    fn test_scale_amplitude() {
        let sample_rate = 44100.0;
        let num_samples = 44100;
        let num_channels = 1;
        let amplitude = 0.5;
        let multiplier: Sample = 0.33;

        let clip = AudioClip::new_monoamplitude(sample_rate, num_samples, amplitude, num_channels);
        let new_clip = clip.scale_amplitude(multiplier);

        assert!(new_clip.num_channels > 0);
        assert_eq!(new_clip.num_samples, clip.num_samples);
        assert_eq!(new_clip.channels[0].len(), clip.num_samples);
        assert!(new_clip.channels[0].iter().all(|v| *v == amplitude * multiplier));
    }

    #[test]
    fn test_add_mut() {
        let sample_rate = 44100.0;
        let num_samples = 1000;
        let amplitude_0 = 0.25;
        let amplitude_1 = 0.5;

        let mut clip_0 = AudioClip::new_monoamplitude(sample_rate, num_samples, amplitude_0, 1);
        let clip_1 = AudioClip::new_monoamplitude(sample_rate, num_samples, amplitude_1, 1);
        clip_0.add_mut(&clip_1, 1.0);

        assert!(clip_0.channels[0].iter().all(|v| *v == amplitude_0 + amplitude_1));
    }
}