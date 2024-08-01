mod audio_clip;
mod resample;

use audio_clip::*;

use std::fs;
use std::path::PathBuf;

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

pub fn run(source: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let tetris_sounds_orig = PathBuf::from("assets_sound");
    let tetris_sounds_resampled = PathBuf::from("tmp_assets_sound");

    let clip = AudioClip::new(source)?;

    // find important metadata
    let orig_tetris_clips = TetrisClips::new(&tetris_sounds_orig)?;
    let mut max_duration = clip.duration;
    let mut max_sample_rate = clip.sample_rate;
    for clip in orig_tetris_clips.clips {
        // f64 doesn't support ord, only partial-ord
        if clip.duration > max_duration {
            max_duration = clip.duration;
        }
        if clip.sample_rate > max_sample_rate {
            max_sample_rate = clip.sample_rate;
        }
    }

    // standardize tetris clips; this makes later comparisons of clips easier
    resample::run_dir(&tetris_sounds_orig, &tetris_sounds_resampled, max_sample_rate)?;
    let new_tetris_clips = TetrisClips::new(&tetris_sounds_resampled)?;
    for clip in new_tetris_clips.clips {
        println!("{:?}", clip);
    }

    // now split the input
    let clip = InputAudioClip::new(source, max_duration)?;

    // cleanup
    fs::remove_dir_all(tetris_sounds_resampled)?;

    Ok(())
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
                channels.push(Vec::new());
                for sample_idx in chunk_indices {
                    channels.last_mut().unwrap().push(clip.channels[channel_idx][*sample_idx]);
                }
            }
            let duration = chunk_indices.len() as f64 / clip.sample_rate;

            // create the audio clip once we have all the channels
            chunks.push(
                AudioClip {
                    channels,
                    duration,
                    file_name: clip.file_name.clone(),
                    sample_rate: clip.sample_rate,
                    max_amplitude: clip.max_amplitude,
                    num_channels: clip.num_channels,
                    num_samples: chunk_num_samples,
                }
            )
        }

        Ok(InputAudioClip{chunks})
    }
}

#[cfg(test)]
mod tests {
    use std::path;

    use super::*;

    #[test]
    fn test_split_input_audio_clip() {
        let duration = 0.2;
        let source = path::PathBuf::from("test_sources/a6.mp3");
        let clip = InputAudioClip::new(&source, duration).expect("failed to create audio clip");

        assert_eq!(clip.chunks.len(), 15);

        // exclude last since rounding errors
        for chunk in clip.chunks.iter().take(clip.chunks.len() - 1) {
            assert_eq!(chunk.duration, duration);
        }
        assert_ne!(clip.chunks.last().unwrap().duration, duration);
    }

    #[test]
    fn test_tetris_clips() {
        let source = path::PathBuf::from("test_sources");
        let tetris_clips = TetrisClips::new(&source).expect("failed to create tetris clips");
        
        assert_eq!(tetris_clips.clips.len(), 7);
    }
}