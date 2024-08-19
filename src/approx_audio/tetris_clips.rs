use crate::approx_audio::pitch::NoteTracker;

use super::audio_clip::{AudioClip, Sample};
use super::fft::FFTResult;

use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;

#[derive(Debug)]
pub struct TetrisClips {
    pub clips: Vec<AudioClip>
}

impl TetrisClips {
    pub fn new(source: &Path) -> Result<TetrisClips> {
        let mut clips = Vec::new();
        for path in source.read_dir()? {
            let path = path?;
            let clip = AudioClip::new(&path.path())?;

            match path.file_name() {
                // combotones are made of multiple clips, not just one
                name if name == "comboTones.mp3" || name == "comboTones.wav" => {
                    let combotones = TetrisClips::split_combotones(&clip);
                    let chromatic_combos = TetrisClips::extrapolate_chromatic_notes(&combotones);
                    clips.extend(chromatic_combos);
                },
                _ => (),
            }
        }

        Ok(TetrisClips { clips })
    }

    #[allow(clippy::cast_precision_loss)]
    fn split_combotones(clips: &AudioClip) -> Vec<AudioClip> {
        const NUM_COMBOS: usize = 15;
        let combo_duration = clips.duration / NUM_COMBOS as f64;

        // there may be an extra combo due to rounding errors; drop it 
        let combos = clips.split_by_duration(combo_duration);
        combos.into_iter().take(NUM_COMBOS).collect()
    }

    fn extrapolate_chromatic_notes(combotones: &Vec<AudioClip>) -> Vec<AudioClip> {
        // three lined bsharp/c
        const MAX_FREQ: Sample = 1046.50;
        const NOTES_IN_COMBOTONES_OCTAVE: usize = 6;
        let chromatic_diff = Sample::from(2.0).powf(1.0 / 12.0);
        assert_eq!(combotones.len(), 15);

        // set up the iterators to loop through existing combotones to pitchshift new ones
        let combotones_fft: Vec<FFTResult> = combotones
            .iter()
            .map(|clip| clip.fft())
            .collect();
        let combotones_freq = combotones_fft
            .iter()
            .map(|fft| fft.most_significant_frequency())
            .collect_vec();
        let freq_fft_iter = combotones_freq.iter().zip(combotones_fft.iter());
        let mut lower_notes_iter = freq_fft_iter.clone().take(NOTES_IN_COMBOTONES_OCTAVE).cycle();

        // prevents new notes being created for notes that already exists
        let mut note_tracker = NoteTracker::new();
        for &freq in &combotones_freq {
            note_tracker.add_note(freq).expect("failed to add note");
        }

        let mut curr_freq = combotones_freq[0] / (Sample::from(2.0).powf(2.0));
        let mut final_clips = Vec::new();
        loop {
            let (&freq, fft) = lower_notes_iter.next().unwrap();

            // limit check
            curr_freq *= chromatic_diff;
            if curr_freq > MAX_FREQ {
                break;
            }
            if note_tracker.contains_note(curr_freq) {
                continue;
            }

            // finally, create the pitch shifted note
            let multiplier = curr_freq / freq;
            final_clips.push(fft.pitch_shift(multiplier).ifft_to_audio_clip());
        }

        final_clips
    }

    #[allow(dead_code)]
    pub fn dump(&self, output_dir: &Path) -> Result<()> {
        for clip in &self.clips {
            let clip_path = PathBuf::from(clip.file_name.clone());
            let clip_file_name = clip_path.file_name().unwrap();
            clip.write(Some(&output_dir.join(clip_file_name)))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path;

    use super::*;

    #[test]
    #[ignore]
    fn test_tetris_clips() {
        let source = path::Path::new("test_audio_clips");
        let tetris_clips = TetrisClips::new(&source).expect("failed to create tetris clips");

        for clip in tetris_clips.clips.iter() {
            assert_eq!(clip.num_samples, clip.channels[0].len());
            for channel in clip.channels.iter() {
                assert_eq!(channel.len(), clip.num_samples);
            }
        }

        assert_ne!(tetris_clips.clips.len(), 0);
    }

    #[test]
    fn test_combotones() {
        let source = path::Path::new("test_audio_clips/comboTones.mp3");
        let combotones = AudioClip::new(&source).expect("failed to create audio clip");
        let split_combotones = TetrisClips::split_combotones(&combotones);

        assert_eq!(split_combotones.len(), 15);
        assert!(split_combotones.iter().all(|clip| clip.num_channels == clip.channels.len()));
        assert!(split_combotones.iter().all(|clip| clip.num_samples == clip.channels[0].len()));
    }

    #[test]
    fn test_combotones_notetracker() {
        let source = path::Path::new("test_audio_clips/comboTones.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let combotones = TetrisClips::split_combotones(&clip);
        let combotones_fft = combotones.iter().map(|clip| clip.fft()).collect_vec();
        let frequencies = combotones_fft.iter().map(|fft| fft.most_significant_frequency()).collect_vec();

        let mut note_tracker = NoteTracker::new();
        for &freq in &frequencies {
            note_tracker.add_note(freq).expect("failed to add note"); 
        }

        for &freq in &frequencies {
            assert!(note_tracker.contains_note(freq), "note not found: {}", freq);
        }
    }

    #[test]
    #[ignore]
    fn test_combotones_chromatic() {
        let source = path::Path::new("test_audio_clips/comboTones.mp3");
        let output = path::Path::new("test_chromatic_tones.wav");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let combotones = TetrisClips::split_combotones(&clip);
        let chromatic_notes = TetrisClips::extrapolate_chromatic_notes(&combotones);

        let mut final_clip = chromatic_notes[0].clone();
        for clip in chromatic_notes.iter().skip(1) {
            final_clip.append_mut(clip);
        }
        final_clip.write(Some(&output)).expect("failed to write final clip");
    }
}