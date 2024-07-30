use std::path::PathBuf;
use std::sync::Arc;
use std::fmt;

use fundsp::prelude::*;

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

struct AudioClip {
    samples: Vec<Vec<f32>>,
    file_name: String,
    sample_rate: f64,
    max_amplitude: f32,
    num_channels: usize,
    num_samples: usize,
}

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
    pub fn new(source: &PathBuf) -> Result<InputAudioClip, Box<dyn std::error::Error>> {
        let clip = AudioClip::new(source)?;
        todo!()
    }
}

impl AudioClip {
    pub fn new(source: &PathBuf) -> Result<AudioClip, Box<dyn std::error::Error>> {
        let wave = Wave::load(source)?;
        let sample_rate = wave.sample_rate();
        let max_amplitude = wave.amplitude();
        let num_channels = wave.channels();
        let num_samples: usize = (wave.duration() * wave.sample_rate()) as usize;
        let mut samples: Vec<Vec<f32>> = Vec::new();
        for channel in 0..num_channels {
            let mut channel_samples = Vec::new();
            for index in 0..num_samples {
                channel_samples.push(wave.at(channel, index));
            }
            samples.push(channel_samples);
        }

        Ok(AudioClip {
            samples,
            file_name: source.to_str().unwrap().to_string(),
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
        let _ = AudioClip::new(&source).expect("failed to create audio clip");
    }
}