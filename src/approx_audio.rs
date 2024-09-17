mod audio_clip;
mod fft;
mod onset_detect;
mod pitch;
mod score;
mod tetris_clips;
mod resample;
mod windowing;

use audio_clip::{AudioClip, Sample};
use fft::separate_harmonic_percussion;
use pitch::CHROMATIC_MULTIPLIER;
use tetris_clips::TetrisClips;
use crate::utils::progress_bar;

use std::fs;
use std::path::Path;
use std::cmp;
use std::collections::BinaryHeap;

use anyhow::Result;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use rust_lapper::{Lapper, Interval};

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
        clip.audio.add_new_channels_mut(max_channels);
    }

    // now split the input
    let clip = InputAudioClip::new(source_resampled, max_channels)?;
    let approx_clip = clip.approx(&tetris_clips)?;
    let final_clip = approx_clip.to_audio_clip();
    final_clip.write(Some(output))?;

    let source_clip = AudioClip::new(source_resampled)?;
    println!("Final MSE: {}", final_clip.mse(&source_clip, 1.0));
    println!("Final Dot: {}", final_clip.dot_product(&source_clip, 1.0));

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
    for clip in orig_tetris_clips.clips.iter().map(|clip| &clip.audio) {
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
        let clip = AudioClip::new(source)?;

        println!("Separating harmonic and percussive components...");
        let window_size = 1024;
        let hop_size = window_size / 4;
        let (harmonic_clip, _percussion_clip) = separate_harmonic_percussion(&clip, window_size, hop_size);

        // harmonic_clip.write(Some(Path::new("tmp_harmonic.wav")))?;
        // percussion_clip.write(Some(Path::new("tmp_percussion.wav")))?;

        // standardizing for original clip
        let mut harmonic_clip = harmonic_clip.resize(clip.num_samples);
        let multiplier = clip.rms_magnitude() / harmonic_clip.rms_magnitude();
        if multiplier.is_finite() {
            harmonic_clip = harmonic_clip.scale_amplitude(multiplier as Sample);
        }
        harmonic_clip.add_new_channels_mut(num_channels);

        let chunks = harmonic_clip.split_by_onsets();
        Ok(InputAudioClip{chunks})
    }

    pub fn approx(&self, tetris_clips: &TetrisClips) -> Result<Self> {
        let pb = progress_bar(self.chunks.len())?;
        pb.set_message("Approximating audio chunks...");
        let output_clips: Vec<AudioClip> = self.chunks
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

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
    fn approx_chunk(chunk: &AudioClip, tetris_clips: &TetrisClips) -> AudioClip {
        let mut output = AudioClip::new_monoamplitude(chunk.sample_rate, chunk.num_samples, 0.0, chunk.num_channels);

        // heap contains (magnitude, frequency)
        let chunk_fft = chunk.fft();
        let mut fft_samples: Vec<(OrderedFloat<Sample>, OrderedFloat<Sample>, OrderedFloat<Sample>)> = Vec::new();
        for (freq, samples) in chunk_fft.iter_zip_bins() {
            let magnitude = samples.iter().fold(0.0, |a, &b| a + b.norm());
            // score is used to account for how higher notes take up more frequency bins and scale logarithmically
            let score = freq.ln() * magnitude;
            fft_samples.push((OrderedFloat(score), OrderedFloat(magnitude), OrderedFloat(freq)));
        }
        let mut heap = BinaryHeap::from(fft_samples);
        let max_score = heap.peek().unwrap_or(&(OrderedFloat(0.0), OrderedFloat(0.0), OrderedFloat(0.0))).0;

        // track added notes
        let mut curr_note_tracker: Lapper<usize, usize> = Lapper::new(Vec::new());
        while let Some((score, _mag, freq)) = heap.pop() {
            if score < max_score / 3.0 || score == 0.0 {
                break; 
            }

            let freq = freq.0 as usize;
            let curr_note_res: Vec<_> = curr_note_tracker.find(freq, freq + 1).collect();
            if !curr_note_res.is_empty() {
                continue;
            }

            let note_clip = tetris_clips.get_combotone(freq);
            if let Some((note_clip, _)) = note_clip {
                let start = (freq as Sample / CHROMATIC_MULTIPLIER) as usize;
                let stop = (freq as Sample * CHROMATIC_MULTIPLIER) as usize;
                let interval = Interval { start, stop, val: 0 };
                let multiplier = *(score / max_score);
                output.add_mut(&note_clip.audio, multiplier);
                curr_note_tracker.insert(interval);
            }
        }

        // scale output to the volume of the input chunk
        let multiplier = chunk.rms_magnitude() / output.rms_magnitude();
        let output = if multiplier.is_finite() {
            output.scale_amplitude(multiplier as Sample)
        } else {
            output
        };

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
    #[ignore]
    fn approx_chunk_tones_1() {
        let tone_ids = vec![0, 1];
        test_chunk_tones(&tone_ids);
        
        let tone_ids = vec![0, 5];
        test_chunk_tones(&tone_ids);
        
        let tone_ids = vec![0, 8];
        test_chunk_tones(&tone_ids);
        
        let tone_ids = vec![0, 10];
        test_chunk_tones(&tone_ids);
        
        let tone_ids = vec![0, 10, 25];
        test_chunk_tones(&tone_ids);
    }

    fn test_chunk_tones(tone_ids: &Vec<usize>) {
        let source = Path::new("test_audio_clips");
        let tetris_clips = TetrisClips::new(source).unwrap();

        let first = &tetris_clips.clips[tone_ids[0]].audio;
        let mut chord = AudioClip::new_monoamplitude(first.sample_rate, first.num_samples, 0.0, first.num_channels);
        for &tone_id in tone_ids {
            chord.add_mut(&tetris_clips.clips[tone_id].audio, 1.0);
            println!("fundamental: {}", tetris_clips.clips[tone_id].fft.most_significant_frequency());
        }

        let approx_chunk = InputAudioClip::approx_chunk(&chord, &tetris_clips);

        assert_eq!(approx_chunk.num_channels, chord.num_channels);
        assert_eq!(approx_chunk.sample_rate, chord.sample_rate);
        assert_eq!(approx_chunk.num_samples, chord.num_samples);
        assert_eq!(approx_chunk.duration, chord.duration);

        let mse = chord.mse(&approx_chunk, 1.0);
        assert!(mse < 0.001, "mse: {}", mse);
    }
}
