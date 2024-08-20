mod audio_clip;
mod fft;
mod onset_detect;
mod pitch;
mod score;
mod tetris_clips;
mod resample;
mod windowing;

use audio_clip::{AudioClip, Sample};
use pitch::NoteTracker;
use tetris_clips::TetrisClips;
use crate::utils::progress_bar;

use std::fs;
use std::path::Path;
use std::collections::BinaryHeap;
use std::cmp;

use anyhow::Result;
use rayon::prelude::*;
use ordered_float::OrderedFloat;

#[derive(Clone, Debug)]
struct InputAudioClip {
    chunks: Vec<AudioClip>,
}

struct MetaData {
    max_sample_rate: f64,
    max_channels: usize,
}

pub fn run(source: &Path, output: &Path) -> Result<()> {
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
        clip.add_new_channels_mut(max_channels);
    }

    // now split the input
    let clip = InputAudioClip::new(source_resampled, max_channels)?;
    let approx_clip = clip.approx(&tetris_clips)?;
    let source_clip = AudioClip::new(source_resampled)?;
    let final_clip = approx_clip.to_audio_clip();
    println!("Final MSE: {}", final_clip.mse(&source_clip, 1.0));
    println!("Final Dot: {}", final_clip.dot_product(&source_clip, 1.0));
    final_clip.write(Some(output))?;

    // cleanup
    println!("Cleaning up...");
    cleanup(tetris_sounds_resampled, source_resampled)?;

    Ok(())
}

fn init(source: &Path, tetris_sounds: &Path) -> Result<MetaData> {
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

fn cleanup(tetris_sounds_resampled: &Path, input_resampled: &Path) -> Result<()> {
    fs::remove_dir_all(tetris_sounds_resampled)?;
    fs::remove_file(input_resampled)?;
    Ok(())
}

impl InputAudioClip {
    pub fn new(source: &Path, num_channels: usize) -> Result<InputAudioClip> {
        let mut clip = AudioClip::new(source)?;
        clip.add_new_channels_mut(num_channels);
        let chunks = clip.split_by_onsets();
        Ok(InputAudioClip{chunks})
    }

    pub fn approx(&self, tetris_clips: &TetrisClips) -> Result<Self> {
        let pb = progress_bar(self.chunks.len())?;
        pb.set_message("Approximating audio chunks...");
        let output_clips = self.chunks
            .par_iter()
            .map(|chunk| {
                let approx_chunk = Self::approx_chunk(chunk, tetris_clips);
                pb.inc(1);
                approx_chunk
                })
            .collect();
        pb.finish_with_message("Done approximating audio chunks!");

        Ok(Self { chunks: output_clips })
    }

    fn approx_chunk(chunk: &AudioClip, tetris_clips: &TetrisClips) -> AudioClip {
        let mut output = AudioClip::new_monoamplitude(chunk.sample_rate, chunk.num_samples, 0.0, chunk.num_channels);

        // take magnitudes of different frequencies one by one
        let chunk_fft = chunk.fft();

        // heap contains (magnitude, frequency)
        let mut fft_samples: Vec<(OrderedFloat<Sample>, OrderedFloat<Sample>)> = Vec::new();
        for (freq, samples) in chunk_fft.iter_zip_bins() {
            let magnitude = samples.iter().fold(0.0, |a, &b| a + b.norm());
            fft_samples.push((OrderedFloat(magnitude), OrderedFloat(freq)));
        }
        let mut heap = BinaryHeap::from(fft_samples);
        let max_magnitude = heap.peek().unwrap_or(&(OrderedFloat(0.0), OrderedFloat(0.0))).0;

        // track added notes
        let mut curr_note_tracker = NoteTracker::new();

        while let Some((mag, freq)) = heap.pop() {
            if mag < max_magnitude / 2.0 {
                break; 
            }

            if curr_note_tracker.get_note(freq.0) != None {
                continue;
            }

            let note_clip = tetris_clips.get_combotone(freq.0);
            match note_clip {
                Some(note_clip) => {
                    output.add_mut(note_clip, 1.0);
                    curr_note_tracker.add_note(freq.0, 0);
                },
                None => (),
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
    use std::path::Path;

    use super::*;

    #[test]
    fn test_split_input_to_audio_clip() {
        let source = Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let input_clip = InputAudioClip::new(&source, clip.num_channels).expect("failed to create audio clip").to_audio_clip();

        assert_eq!(input_clip.num_channels, clip.num_channels);
        assert_eq!(input_clip.sample_rate, clip.sample_rate);
        assert_eq!(input_clip.num_samples, clip.num_samples);
        assert_eq!(input_clip.duration, clip.duration);
    }

    #[test]
    fn approx_chunk_tones_1() {
        let tone_ids = vec![0, 1];
        test_chunk_tones(&tone_ids);
    }

    #[test]
    fn approx_chunk_tones_2() {
        let tone_ids = vec![0, 5];
        test_chunk_tones(&tone_ids);
    }

    #[test]
    fn approx_chunk_tones_3() {
        let tone_ids = vec![0, 8];
        test_chunk_tones(&tone_ids);
    }

    #[test]
    fn approx_chunk_tones_4() {
        let tone_ids = vec![0, 10];
        test_chunk_tones(&tone_ids);
    }

    #[test]
    fn approx_chunk_tones_5() {
        let tone_ids = vec![0, 10, 25];
        test_chunk_tones(&tone_ids);
    }

    fn test_chunk_tones(tone_ids: &Vec<usize>) {
        let source = Path::new("test_audio_clips");
        let tetris_clips = TetrisClips::new(source).unwrap();

        let first = &tetris_clips.clips[tone_ids[0]];
        let mut chord = AudioClip::new_monoamplitude(first.sample_rate, first.num_samples, 0.0, first.num_channels);
        for &tone_id in tone_ids {
            let clip = &tetris_clips.clips[tone_id];
            let clip_fft = clip.fft();

            println!("id: {tone_id}, most significant freq: {}", clip_fft.most_significant_frequency());

            chord.add_mut(&tetris_clips.clips[tone_id], 1.0);
        }

        let approx_chunk = InputAudioClip::approx_chunk(&chord, &tetris_clips);

        assert_eq!(approx_chunk.num_channels, chord.num_channels);
        assert_eq!(approx_chunk.sample_rate, chord.sample_rate);
        assert_eq!(approx_chunk.num_samples, chord.num_samples);
        assert_eq!(approx_chunk.duration, chord.duration);

        let mse = chord.mse(&approx_chunk, 1.0);
        assert!(mse == 0.0, "mse: {}", mse);
    }
}
