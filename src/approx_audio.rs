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

struct AudioClip {
    wave: Wave,
    sample_rate: f64,
    max_amplitude: f32,
    num_channels: usize,
    num_samples: usize,
}

pub fn run(source: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let clip = AudioClip::new(source)?;
    println!("{:?}", clip);
    todo!();
}

impl AudioClip {
    pub fn new(source: &PathBuf) -> Result<AudioClip, Box<dyn std::error::Error>> {
        let wave = Wave::load(source)?;
        let sample_rate = wave.sample_rate();
        let max_amplitude = wave.amplitude();
        let num_channels = wave.channels();
        let num_samples: usize = (wave.duration() * wave.sample_rate()) as usize;
        Ok(AudioClip {
            wave,
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