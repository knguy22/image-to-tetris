mod audio_clip;
mod resample;

use audio_clip::*;

use std::fs;
use std::path::PathBuf;
use std::cmp;

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

#[derive(Clone, Debug)]
struct InputAudioClip {
    chunks: Vec<AudioClip>,
}

struct MetaData {
    max_duration: f64,
    max_sample_rate: f64,
}

pub fn run(source: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let tetris_sounds_orig = PathBuf::from("assets_sound");
    let tetris_sounds_resampled = PathBuf::from("tmp_assets_sound");

    let MetaData{max_duration, max_sample_rate} = init(source, &tetris_sounds_orig)?;
    println!("Approximating audio with sample rate {} and duration {}", max_sample_rate, max_duration);

    // standardize tetris clips; this makes later comparisons of clips easier
    resample::run_dir(&tetris_sounds_orig, &tetris_sounds_resampled, max_sample_rate)?;
    let new_tetris_clips = TetrisClips::new(&tetris_sounds_resampled)?;

    // now split the input
    let clip = InputAudioClip::new(source, max_duration)?;
    let approx_clip = clip.approx(&new_tetris_clips);
    match approx_clip {
        Ok(approx_clip) => approx_clip.to_audio_clip().write(&output)?,
        _ => (),
    }

    // cleanup
    cleanup(&tetris_sounds_resampled)?;

    Ok(())
}

fn init(source: &PathBuf, tetris_sounds: &PathBuf) -> Result<MetaData, Box<dyn std::error::Error>> {
    let clip = AudioClip::new(source)?;

    // find important metadata
    let orig_tetris_clips = TetrisClips::new(&tetris_sounds)?;
    let mut max_duration = 0.0;
    let mut max_sample_rate = clip.sample_rate;
    for clip in orig_tetris_clips.clips {
        // f64 doesn't support ord, only partial-ord which is why max is not used
        if clip.duration > max_duration {
            max_duration = clip.duration;
        }
        if clip.sample_rate > max_sample_rate {
            max_sample_rate = clip.sample_rate;
        }
    }

    Ok(MetaData {
        max_duration,
        max_sample_rate,
    })
}

fn cleanup(tetris_sounds_resampled: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
            // also keep track of metadata along the way
            let mut channels = Vec::new();
            let mut max_amplitude = 0.0;
            for channel_idx in 0..clip.num_channels {
                channels.push(Vec::new());
                for sample_idx in chunk_indices {
                    channels.last_mut().unwrap().push(clip.channels[channel_idx][*sample_idx]);

                    if clip.channels[channel_idx][*sample_idx] > max_amplitude {
                        max_amplitude = clip.channels[channel_idx][*sample_idx];
                    }
                }
            }

            let num_samples = chunk_indices.len();
            let duration = num_samples as f64 / clip.sample_rate;

            // create the audio clip once we have all the channels
            chunks.push(
                AudioClip {
                    channels,
                    duration,
                    file_name: clip.file_name.clone(),
                    sample_rate: clip.sample_rate,
                    max_amplitude,
                    num_channels: clip.num_channels,
                    num_samples,
                }
            )
        }

        Ok(InputAudioClip{chunks})
    }

    pub fn approx(&self, tetris_clips: &TetrisClips) -> Result<Self, Box<dyn std::error::Error>> {
        let mut output = self.clone();

        for (chunk_idx, chunk) in self.chunks.iter().enumerate() {
            // choose a best tetris clip for the specific chunk
            let best_clip: &AudioClip = &tetris_clips.clips[0];
            for clip in tetris_clips.clips.iter().skip(1) {
                continue;
            }

            assert!(chunk.num_channels == best_clip.num_channels);

            // prevent index overflow since the last chunk can be smaller than the others
            let num_samples_to_write = cmp::min(chunk.num_samples, best_clip.num_samples);

            // then overwrite the best clip to the output
            for channel_idx in 0..best_clip.num_channels {
                assert!(chunk.num_samples == output.chunks[chunk_idx].channels[channel_idx].len());
                assert!(best_clip.num_samples == best_clip.channels[channel_idx].len());

                for sample_idx in 0..num_samples_to_write {
                    output.chunks[chunk_idx].channels[channel_idx][sample_idx] = best_clip.channels[channel_idx][sample_idx];
                }

                // let extra samples not covered by best clip be 0
                for sample_idx in num_samples_to_write..chunk.num_samples {
                    output.chunks[chunk_idx].channels[channel_idx][sample_idx] = 0.0;
                }
            }
        }

        Ok(output)
    }

    // joins all the contained chunks into a single audio clip
    pub fn to_audio_clip(&self) -> AudioClip {
        let mut channels: Vec<Vec<f32>> = vec![Vec::new(); self.chunks[0].num_channels];
        for chunk in &self.chunks {
            for channel_idx in 0..chunk.num_channels {
                channels[channel_idx].extend(&chunk.channels[channel_idx]);
            }
        }

        let num_samples = channels[0].len();
        let duration = channels[0].len() as f64 / self.chunks[0].sample_rate;

        AudioClip {
            channels,
            duration,
            file_name: String::new(),
            sample_rate: self.chunks[0].sample_rate,
            max_amplitude: 0.0,
            num_channels: self.chunks[0].num_channels,
            num_samples,
        }


    }
}

#[cfg(test)]
mod tests {
    use std::path;

    use super::*;

    #[test]
    fn test_split_input_clip() {
        let duration = 0.2;
        let source = path::PathBuf::from("test_sources/a6.mp3");
        let clip = InputAudioClip::new(&source, duration).expect("failed to create audio clip");

        assert_eq!(clip.chunks.len(), 15);

        // exclude last due to rounding errors
        for chunk in clip.chunks.iter().take(clip.chunks.len() - 1) {
            assert_eq!(chunk.duration, duration);
            assert_eq!(chunk.num_samples, chunk.channels[0].len());

            for channel in chunk.channels.iter() {
                assert_eq!(channel.len(), chunk.num_samples);
            }
        }

        let last = clip.chunks.last().unwrap();
        assert_ne!(last.duration, duration);
        assert_eq!(last.num_samples, last.channels[0].len());
    }

    #[test]
    fn test_split_input_to_audio_clip() {
        let source = path::PathBuf::from("test_sources/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let input_clip = InputAudioClip::new(&source, 0.1).expect("failed to create audio clip").to_audio_clip();

        assert_eq!(input_clip.num_channels, clip.num_channels);
        assert_eq!(input_clip.sample_rate, clip.sample_rate);
        assert_eq!(input_clip.num_samples, clip.num_samples);
        assert_eq!(input_clip.duration, clip.duration);
    }

    #[test]
    fn test_tetris_clips() {
        let source = path::PathBuf::from("test_sources");
        let tetris_clips = TetrisClips::new(&source).expect("failed to create tetris clips");

        for clip in tetris_clips.clips.iter() {
            assert_eq!(clip.num_samples, clip.channels[0].len());
            for channel in clip.channels.iter() {
                assert_eq!(channel.len(), clip.num_samples);
            }
        }
        
        assert_eq!(tetris_clips.clips.len(), 7);
    }
}