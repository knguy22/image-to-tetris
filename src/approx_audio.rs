use std::path::PathBuf;
use std::fmt;

use fundsp::prelude::*;
use itertools::Itertools;
use rustfft::{FftPlanner, num_complex::Complex};

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
    channels: Vec<Channel>,
    file_name: String,
    sample_rate: f64,
    duration: f64,
    max_amplitude: f32,
    num_channels: usize,
    num_samples: usize,
}

struct FFTResult {
    channels: Vec<FFTChannel>,
    frequency_resolution: f64,
}

type Channel = Vec<Sample>;
type Sample = f32;
type FFTChannel = Vec<FFTSample>;
type FFTSample = Complex<Sample>;

pub fn run(source: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let clip = AudioClip::new(source)?;
    println!("{:?}", clip);

    let fft_res = clip.fft();
    println!("{:?}", fft_res);
    dump_fft_to_csv(&fft_res)?;

    // let tetris_clips = TetrisClips::new(&PathBuf::from("assets_sound"))?;
    // println!("{:?}", tetris_clips);
    todo!();
}

impl TetrisClips {
    pub fn new(source: &PathBuf) -> Result<TetrisClips, Box<dyn std::error::Error>> {
        let mut clips = Vec::new();
        for clip in source.read_dir()? {
            let clip = clip?;
            clips.push(AudioClip::new(&clip.path())?);
        }
        Ok(TetrisClips { clips })
    }

}

impl InputAudioClip {
    pub fn new(source: &PathBuf, max_clip_duration: f64) -> Result<InputAudioClip, Box<dyn std::error::Error>> {
        let clip = AudioClip::new(source)?;

        // split the original video into chunks; this will be useful for approximation later
        let mut chunks = Vec::new();
        let sample_indicies = (0..clip.num_samples).into_iter().collect_vec();
        let chunk_num_samples = (max_clip_duration * clip.sample_rate) as usize;
        for chunk_indices in sample_indicies.chunks(chunk_num_samples) {

            // grab each channel one by one at the specified chunk indices
            let mut channels = Vec::new();
            for channel_idx in 0..clip.num_channels {
                let mut channel = Vec::new();
                for sample_idx in chunk_indices {
                    channel.push(clip.channels[channel_idx][*sample_idx]);
                }
                channels.push(channel);
            }

            // create the audio clip once we have all the channels
            chunks.push(
                AudioClip {
                    channels,
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

#[allow(dead_code)]
fn dump_fft_to_csv(fft: &FFTResult) -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from("python/fft.csv");
    let mut wtr = csv::Writer::from_path(output)?;
    wtr.write_record(&["channel", "frequency", "norm"])?;
    for (channel_idx, channel) in fft.channels.iter().enumerate() {
        for (i, sample) in channel.iter().enumerate() {
            let frequency = fft.frequency_resolution * i as f64;
            wtr.write_record(&[channel_idx.to_string(), frequency.to_string(), sample.norm().to_string()])?;
        }
    }

    Ok(())
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

    #[test]
    fn test_split_input_audio_clip() {
        let source = path::PathBuf::from("test_sources/a6.mp3");
        let clip = InputAudioClip::new(&source, 0.2).expect("failed to create audio clip");
        
        assert_eq!(clip.chunks.len(), 15);
    }

    #[test]
    fn test_tetris_clips() {
        let source = path::PathBuf::from("test_sources");
        let tetris_clips = TetrisClips::new(&source).expect("failed to create tetris clips");
        
        assert_eq!(tetris_clips.clips.len(), 7);
    }
}