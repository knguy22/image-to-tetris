use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use fundsp::prelude::*;
use hound::{WavWriter, WavSpec, SampleFormat};

pub type Channel = Vec<Sample>;
pub type Sample = f32;
// the fundamental structure of an audio clip in this project
#[derive(Clone)]
pub struct AudioClip {
    pub channels: Vec<Channel>,
    pub file_name: String,
    pub duration: f64,
    pub sample_rate: f64,
    pub max_amplitude: f32,
    pub num_channels: usize,
    pub num_samples: usize,
}

impl AudioClip {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn new(source: &Path) -> Result<Self, Box<dyn std::error::Error>> {
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
            duration,
            sample_rate,
            max_amplitude,
            num_channels,
            num_samples,
        })
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, dead_code)]
    pub fn new_monotone(sample_rate: f64, duration: f64, amplitude: Sample, num_channels: usize) -> Self {
        let num_samples = (duration * sample_rate) as usize;
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
    pub fn write(&self, path: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.unwrap();

        // output file must be wav
        if path.extension().unwrap() != "wav" {
            return Err("output file must be wav".into());
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

    // takes a window of the audio clip
    // pads the window with 0s if the window extends out of bounds
    #[allow(clippy::cast_precision_loss)]
    pub fn window(&self, start: usize, end: usize) -> Self {
        let mut channels = Vec::new();
        for channel in &self.channels {
            let end_in_range = std::cmp::min(end, channel.len());
            let mut to_push = channel[start..end_in_range].to_vec();
            to_push.resize(end - start, 0.0);
            channels.push(to_push);
        }
        let file_name = format!("{}_{}_{}.wav", self.file_name, start, end);
        Self {
            channels,
            file_name,
            duration: (end - start) as f64 / self.sample_rate,
            sample_rate: self.sample_rate,
            max_amplitude: self.max_amplitude,
            num_channels: self.num_channels,
            num_samples: end - start,
        }
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
            let chunk = self.window(begin, end);
            chunks.push(chunk);
        }
        chunks
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn dot_product(&self, other: &Self) -> f64 {
        assert!(self.num_channels == other.num_channels);
        assert!((self.sample_rate - other.sample_rate).abs() < f64::EPSILON);

        let zero_pad_curr = self.num_samples < other.num_samples;
        let curr = if zero_pad_curr {
            self.zero_pad(other.num_samples)
        } else {
            self.clone()
        };

        let other = if zero_pad_curr {
            other.clone()
        } else {
            other.zero_pad(self.num_samples)
        };

        let mut dot_product: f64 = 0.0;
        for channel_idx in 0..curr.num_channels {
            for sample_idx in 0..curr.num_samples {
                let curr_sample = curr.channels[channel_idx][sample_idx];
                let other_sample = other.channels[channel_idx][sample_idx];
                dot_product += f64::from(curr_sample) * f64::from(other_sample);
            }
        }

        assert!(!dot_product.is_nan());
        dot_product / ((curr.num_samples * curr.num_channels) as f64)
    }

    // add new channels to the audio clip
    // uses the average of existing channels for new values
    #[allow(clippy::cast_precision_loss)]
    pub fn add_new_channels(&mut self, num_channels: usize) {
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

    use crate::approx_audio::resample;

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
    fn test_monotone() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let amplitude = 0.5;

        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude, 1);

        assert!(clip.num_channels > 0);
        assert_eq!(clip.num_samples, sample_rate as usize * duration as usize);
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
    fn test_window() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let amplitude = 0.5;

        let start: usize = 1000;
        let end: usize = 8000;
        let window_len = end - start;

        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude, 1);
        let window_clip = clip.window(start, end);

        assert!(window_clip.num_channels > 0);
        assert_eq!(window_clip.num_samples, window_len);
        assert_eq!(window_clip.channels[0].len(), window_len);
        assert!(window_clip.channels[0].iter().all(|v| *v == amplitude));
    }

    #[test]
    fn test_window_overflow() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let amplitude = 0.5;
        let num_samples = sample_rate as usize * duration as usize;

        let start: usize = 44000;
        let end: usize = 46000;
        let window_len = end - start;

        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude, 1);
        let window_clip = clip.window(start, end);

        assert!(window_clip.num_channels > 0);
        assert_eq!(window_clip.num_samples, window_len);
        assert_eq!(window_clip.channels[0].len(), window_len);

        // these samples should still be in range
        assert!(window_clip.channels[0].iter().take(num_samples - start).all(|v| *v == amplitude));

        // these samples should be out of range
        assert!(window_clip.channels[0].iter().skip(num_samples - start).all(|v| *v == 0.0));
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
    fn test_dot_product() {
        // first resample the audio clips to 44100 Hz
        let sample_rate = 44100.0;
        let source_dir = path::Path::new("test_audio_clips");
        let resample_source_dir = path::Path::new("test_resampled_audio_clips");
        resample::run_dir(source_dir, resample_source_dir, sample_rate).expect("failed to resample audio clips");

        // the same file should have the highest dot product with itself
        let mut clips = Vec::new();
        for source in resample_source_dir.read_dir().unwrap() {
            clips.push(AudioClip::new(&source.unwrap().path()).expect("failed to create audio clip"));
        }

        let self_dot_product = clips[0].dot_product(&clips[0]);
        assert!(clips.iter().skip(1).all(|clip| clip.dot_product(&clips[0]) < self_dot_product));

        // cleanup
        fs::remove_dir_all(resample_source_dir).expect("failed to remove resampled audio clips");
    }

    #[test]
    fn test_scale_amplitude() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let amplitude = 0.5;
        let multiplier: f32 = 0.33;

        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude, 1);
        let new_clip = clip.scale_amplitude(multiplier);

        assert!(new_clip.num_channels > 0);
        assert_eq!(new_clip.num_samples, clip.num_samples);
        assert_eq!(new_clip.channels[0].len(), clip.num_samples);
        assert!(new_clip.channels[0].iter().all(|v| *v == amplitude * multiplier));
    }
}