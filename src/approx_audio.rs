use std::path::PathBuf;
use std::fmt;

use fundsp::prelude::*;
use itertools::Itertools;

// Todo:
// Be able to extract audio and metadata into a struct (do this for both input and tetris sample clips)
// Represent the tetris clips inside a single struct
// Split the source into multiple chunks somehow (note detection?)
// approximate each individual chunk
// combine the chunks into a single audio file

#[derive(Debug)]
struct TetrisClips {
    clips: Vec<AudioClip>
}

#[derive(Debug)]
struct InputAudioClip {
    chunks: Vec<AudioClip>,
}

// the fundamental structure of an audio clip in this project
struct AudioClip {
    samples: Vec<Sample>,
    file_name: String,
    sample_rate: f64,
    duration: f64,
    max_amplitude: f32,
    num_channels: usize,
    num_samples: usize,
}

// contains amplitudes for each channel at a certain timestamp
type Sample = Vec<f32>;

pub fn run(source: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let clip = AudioClip::new(source)?;
    println!("{:?}", clip);

    let tetris_clips = TetrisClips::new(source)?;
    println!("{:?}", tetris_clips);
    todo!();
}

impl TetrisClips {
    pub fn new(source: &PathBuf) -> Result<TetrisClips, Box<dyn std::error::Error>> {
        let clips_dir = PathBuf::from("assets_sound");
        let mut clips = Vec::new();
        for clip in clips_dir.read_dir()? {
            let clip = clip?;
            clips.push(AudioClip::new(&clip.path())?);
        }
        Ok(TetrisClips { clips })
    }

}

impl InputAudioClip {
    pub fn new(source: &PathBuf, max_clip_duration: f64) -> Result<InputAudioClip, Box<dyn std::error::Error>> {
        let clip = AudioClip::new(source)?;

        // split the original video into parseable chunks
        let chunk_num_samples = (max_clip_duration * clip.sample_rate) as usize;
        let mut chunks = Vec::new();
        for chunk in clip.samples.chunks(chunk_num_samples) {
            chunks.push(
                AudioClip {
                    samples: chunk.to_vec(),
                    file_name: clip.file_name.clone(),
                    sample_rate: clip.sample_rate,
                    duration: max_clip_duration,
                    max_amplitude: clip.max_amplitude,
                    num_channels: clip.num_channels,
                    num_samples: chunk_num_samples,
                }
            )
        }

        Ok(InputAudioClip{chunks})
    }
}

impl AudioClip {
    pub fn new(source: &PathBuf) -> Result<AudioClip, Box<dyn std::error::Error>> {
        let wave = Wave::load(source)?;
        let sample_rate = wave.sample_rate();
        let duration = wave.duration();
        let max_amplitude = wave.amplitude();
        let num_channels = wave.channels();
        let num_samples: usize = (duration * sample_rate) as usize;
        let mut samples: Vec<Sample> = Vec::new();

        for index in 0..num_samples {
            let mut sample = Sample::new();
            for channel in 0..num_channels {
                sample.push(wave.at(channel, index));
            }
            samples.push(sample);
        }

        Ok(AudioClip {
            samples,
            file_name: source.to_str().unwrap().to_string(),
            duration,
            sample_rate,
            max_amplitude,
            num_channels,
            num_samples,
        })
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
    use std::{fs, path};

    use super::*;

    #[test]
    fn test_create_audio_clip() {
        let source = path::PathBuf::from("test_sources/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");

        assert_ne!(clip.num_channels, 0);
        assert_ne!(clip.num_samples, 0);
        assert_ne!(clip.samples.len(), 0);
    }

    #[test]
    fn test_split_input_audio_clip() {
        let source = path::PathBuf::from("test_sources/a6.mp3");
        let clip = InputAudioClip::new(&source, 0.2).expect("failed to create audio clip");
        
        assert_eq!(clip.chunks.len(), 15);
    }
}