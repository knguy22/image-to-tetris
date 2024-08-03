use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

use fundsp::prelude::*;
use itertools::Itertools;
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
            duration,
            sample_rate,
            max_amplitude,
            num_channels,
            num_samples,
        })
    }

    #[allow(dead_code)]
    pub fn new_monotone(sample_rate: f64, duration: f64, amplitude: Sample) -> Self {
        let num_channels = 1;
        let num_samples = (duration * sample_rate) as usize;
        let mut channels: Vec<Channel> = Vec::new();
        for _ in 0..num_channels {
            let mut channel = Channel::new();
            for sample_idx in 0..num_samples {
                channel.push(amplitude);
            }
            channels.push(channel);
        }

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

    pub fn write(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // output file must be wav
        if path.extension().unwrap() != "wav" {
            return Err("output file must be wav".into());
        }

        // create the output file and initialize the writer
        let spec = WavSpec {
            channels: self.channels.len() as u16,
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
            for channel in self.channels.iter() {
                wav_writer.write_sample(channel[i])?;
            }
        }

        wav_writer.finalize()?;
        Ok(())
    }

    // splits the audio clip into chunks the length of max_duration; if the last chunk is shorter than 
    // max_duration, it will still be included but will be smaller than max_duration
    pub fn split_by_duration(&self, max_duration: f64) -> Vec<Self> {
        // split the original video into chunks; this will be useful for approximation later
        let mut chunks = Vec::new();
        let sample_indicies = (0..self.num_samples).into_iter().collect_vec();
        let chunk_num_samples = (max_duration * self.sample_rate) as usize;
        for (chunk_idx, chunk_indices) in sample_indicies.chunks(chunk_num_samples).enumerate() {

            // grab each channel one by one at the specified chunk indices
            // also keep track of metadata along the way
            let mut channels = Vec::new();
            let mut max_amplitude = 0.0;
            for channel_idx in 0..self.num_channels {
                channels.push(Vec::new());
                for sample_idx in chunk_indices {
                    channels.last_mut().unwrap().push(self.channels[channel_idx][*sample_idx]);

                    if self.channels[channel_idx][*sample_idx] > max_amplitude {
                        max_amplitude = self.channels[channel_idx][*sample_idx];
                    }
                }
            }

            let num_samples = chunk_indices.len();
            let duration = num_samples as f64 / self.sample_rate;
            let file_name = format!("{}_{}.wav", self.file_name, chunk_idx);

            // create the audio clip once we have all the channels
            chunks.push(
                AudioClip {
                    channels,
                    duration,
                    file_name,
                    sample_rate: self.sample_rate,
                    max_amplitude,
                    num_channels: self.num_channels,
                    num_samples,
                }
            )
        }

        chunks
    }

    pub fn dot_product(&self, other: &Self) -> f64 {
        assert!(self.num_channels == other.num_channels);
        assert!(self.sample_rate == other.sample_rate);

        let zero_pad_curr = self.num_samples < other.num_samples;
        let curr = if zero_pad_curr {
            self.zero_pad(other.num_samples).unwrap()
        } else {
            self.clone()
        };

        let other = if zero_pad_curr {
            other.clone()
        } else {
            other.zero_pad(self.num_samples).unwrap()
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

            for new_channel_idx in self.num_channels..num_channels {
                channels[new_channel_idx][sample_idx] = sample_sum / (self.num_channels as Sample);
            }
        }

        self.channels = channels;
        self.num_channels = num_channels;
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

    pub fn append(&self, other: &Self) -> Result<Self, Box<dyn std::error::Error>> {
        assert!(self.sample_rate == other.sample_rate);
        assert!(self.num_channels == other.num_channels);

        let mut clip = self.clone();
        for channel_idx in 0..self.num_channels {
            clip.channels[channel_idx].extend(&other.channels[channel_idx]);
        }
        clip.num_samples = self.num_samples + other.num_samples;
        Ok(clip)
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

#[cfg(test)]
mod tests {
    use std::path;

    use crate::approx_audio::resample;

    use super::*;

    #[test]
    fn test_create_audio_clip() {
        let source = path::PathBuf::from("test_audio_clips/a6.mp3");
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

        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude);

        assert!(clip.num_channels > 0);
        assert_eq!(clip.num_samples, sample_rate as usize * duration as usize);
        assert_eq!(clip.channels[0].len(), clip.num_samples);
        assert!(clip.channels[0].iter().all(|v| *v == amplitude));
    }

    #[test]
    fn test_write_audio_clip() {
        let source = path::PathBuf::from("test_audio_clips/a6.mp3");
        let output = path::PathBuf::from("test_results/test.wav");

        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        clip.write(&output).expect("failed to write audio clip");

        assert!(output.exists());

        // cleanup
        fs::remove_file(output).expect("failed to remove test file");
    }

    #[test]
    fn test_split_clip() {
        let duration = 0.2;
        let source = path::PathBuf::from("test_audio_clips/a6.mp3");
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
        let source = path::PathBuf::from("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let output = clip.zero_pad(num_samples).expect("failed to zero pad audio clip");

        assert_eq!(output.num_samples, num_samples);
        assert!(output.channels.iter().all(|channel| channel.len() == num_samples));
    }

    #[test]
    fn test_dot_product() {
        // first resample the audio clips to 44100 Hz
        let sample_rate = 44100.0;
        let source_dir = path::PathBuf::from("test_audio_clips");
        let resample_source_dir = path::PathBuf::from("test_resampled_audio_clips");
        resample::run_dir(&source_dir, &resample_source_dir, sample_rate).expect("failed to resample audio clips");

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
}