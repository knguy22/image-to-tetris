mod audio_clip;
mod fft;
mod onset_detect;
mod tetris_clips;
mod resample;

use audio_clip::{AudioClip, Sample};
use tetris_clips::TetrisClips;
use crate::utils::progress_bar;

use std::fs;
use std::path::Path;
use std::cmp;

use rayon::prelude::*;

#[derive(Clone, Debug)]
struct InputAudioClip {
    chunks: Vec<AudioClip>,
}

struct MetaData {
    max_sample_rate: f64,
    max_channels: usize,
}

pub fn run(source: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let tetris_sounds_orig = Path::new("assets_sound");
    let tetris_sounds_resampled = Path::new("tmp_tetris_sounds_assets");
    let source_resampled = Path::new("tmp_source.wav");

    let MetaData{max_sample_rate, max_channels} = init(source, tetris_sounds_orig)?;
    println!("Approximating audio with sample rate {max_sample_rate}");

    // standardize tetris clips + input clip; this makes later comparisons of clips easier
    println!("Resampling clips...");
    resample::run_dir(tetris_sounds_orig, tetris_sounds_resampled, max_sample_rate)?;
    resample::run(source, source_resampled, max_sample_rate)?;
    let mut tetris_clips = TetrisClips::new(tetris_sounds_resampled)?;
    for clip in &mut tetris_clips.clips {
        clip.add_new_channels(max_channels);
    }
    tetris_clips.dump(Path::new("results"))?;

    // now split the input
    let clip = InputAudioClip::new(source_resampled, max_channels)?;
    let approx_clip = clip.approx(&tetris_clips)?;
    let source_clip = AudioClip::new(source_resampled)?;
    let final_clip = approx_clip.to_audio_clip();
    let final_approx_score = final_clip.dot_product(&source_clip);
    println!("Approximation score: {final_approx_score}");
    final_clip.write(Some(output))?;

    // cleanup
    println!("Cleaning up...");
    cleanup(tetris_sounds_resampled, source_resampled)?;

    Ok(())
}

fn init(source: &Path, tetris_sounds: &Path) -> Result<MetaData, Box<dyn std::error::Error>> {
    let clip = AudioClip::new(source)?;

    // find important metadata
    let orig_tetris_clips = TetrisClips::new(tetris_sounds)?;
    let mut max_sample_rate = clip.sample_rate;
    let mut max_channels = clip.num_channels;
    for clip in orig_tetris_clips.clips {
        // f64 doesn't support ord, only partial-ord which is why max is not used
        if clip.sample_rate > max_sample_rate {
            max_sample_rate = clip.sample_rate;
        }
        max_channels = cmp::max(max_channels, clip.num_channels);
    }

    Ok(MetaData {
        max_sample_rate,
        max_channels,
    })
}

fn cleanup(tetris_sounds_resampled: &Path, input_resampled: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::remove_dir_all(tetris_sounds_resampled)?;
    fs::remove_file(input_resampled)?;
    Ok(())
}

impl InputAudioClip {
    pub fn new(source: &Path, num_channels: usize) -> Result<InputAudioClip, Box<dyn std::error::Error>> {
        let mut clip = AudioClip::new(source)?;
        clip.add_new_channels(num_channels);
        let chunks = clip.split_by_onsets();
        Ok(InputAudioClip{chunks})
    }

    pub fn approx(&self, tetris_clips: &TetrisClips) -> Result<Self, Box<dyn std::error::Error>> {
        let pb = progress_bar(self.chunks.len())?;

        pb.set_message("Approximating audio chunks...");
        let output_clips = self.chunks
            .par_iter()
            .map(|chunk| {
                let approx_chunk = self.approx_chunk(chunk, tetris_clips);
                pb.inc(1);
                approx_chunk
                })
            .collect();
        pb.finish_with_message("Done approximating audio chunks!");

        Ok(Self { chunks: output_clips })
    }

    fn approx_chunk(&self, chunk: &AudioClip, tetris_clips: &TetrisClips) -> AudioClip {
        let mut output = chunk.clone();

        // choose a best tetris clip for the specific chunk
        let mut best_clip: &AudioClip = &tetris_clips.clips[0];
        let mut best_dot_product = chunk.dot_product(best_clip);
        for clip in tetris_clips.clips.iter().skip(1) {
            let dot_product = chunk.dot_product(clip);
            if dot_product > best_dot_product {
                best_dot_product = dot_product;
                best_clip = clip;
            }
        }

        assert!(chunk.num_channels == best_clip.num_channels);
        assert!((chunk.sample_rate - best_clip.sample_rate).abs() < f64::EPSILON);

        // prevent index overflow since the last chunk can be smaller than the others
        let num_samples_to_write = cmp::min(chunk.num_samples, best_clip.num_samples);

        // then overwrite the best clip to the output
        for channel_idx in 0..best_clip.num_channels {
            assert!(chunk.num_samples == output.channels[channel_idx].len());
            assert!(best_clip.num_samples == best_clip.channels[channel_idx].len());

            for sample_idx in 0..num_samples_to_write {
                output.channels[channel_idx][sample_idx] = best_clip.channels[channel_idx][sample_idx];
            }

            // let extra samples not covered by best clip be 0
            for sample_idx in num_samples_to_write..chunk.num_samples {
                output.channels[channel_idx][sample_idx] = 0.0;
            }
        }

        output
    }

    // joins all the contained chunks into a single audio clip
    #[allow(clippy::cast_precision_loss)]
    pub fn to_audio_clip(&self) -> AudioClip {
        let mut channels: Vec<Vec<Sample>> = vec![Vec::new(); self.chunks[0].num_channels];
        for chunk in &self.chunks {
            for (channel_idx, channel) in channels.iter_mut().enumerate().take(chunk.num_channels) {
                channel.extend(&chunk.channels[channel_idx]);
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
    fn test_split_input_to_audio_clip() {
        let source = path::Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let input_clip = InputAudioClip::new(&source, clip.num_channels).expect("failed to create audio clip").to_audio_clip();

        assert_eq!(input_clip.num_channels, clip.num_channels);
        assert_eq!(input_clip.sample_rate, clip.sample_rate);
        assert_eq!(input_clip.num_samples, clip.num_samples);
        assert_eq!(input_clip.duration, clip.duration);
    }
}