use super::audio_clip::{AudioClip, Sample};
use super::fft::FFTResult;

use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;
use rust_lapper::{Lapper, Interval};

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
                    let combos = TetrisClips::split_combotones(&clip);
                    clips.extend(combos);
                },
                _ => clips.push(clip),
            }
        }

        // songs to split by onsets
        let songs_to_split = Path::new("assets_sound_onset_split");
        if songs_to_split.exists() {
            for path in songs_to_split.read_dir()? {
                let path = path?;
                let clip = AudioClip::new(&path.path())?.split_by_onsets();
                clips.extend(clip);
            }
        }
        else {
            println!("Warning: no songs to split by onsets found");
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

        // let the intervals contain the combotone indices as value so they can be looked up later
        let combotones_intervals: Vec<Interval<usize, usize>> = combotones
            .iter()
            .enumerate()
            .map(|(i, clip)| Self::interval(clip.fft().most_significant_frequency(), i))
            .collect();

        let combotones_fft: Vec<FFTResult> = combotones
            .iter()
            .map(|clip| clip.fft())
            .collect();
        let combotones_freq = combotones_fft
            .iter()
            .map(|fft| fft.most_significant_frequency())
            .collect_vec();
        let freq_fft_iter = combotones_freq.iter().zip(combotones_fft.iter());

        // we will use the lower octave in combotones in order to create the notes pitched downwards
        let mut lower_notes_iter = freq_fft_iter.clone().take(NOTES_IN_COMBOTONES_OCTAVE).cycle();

        let mut final_clips = Vec::new();
        let lapper = Lapper::new(combotones_intervals);
        let mut curr_freq = combotones_freq[0] / (Sample::from(2.0).powf(2.0));
        loop {
            let (&freq, fft) = lower_notes_iter.next().unwrap();

            // limit check
            curr_freq *= chromatic_diff;
            if curr_freq > MAX_FREQ {
                break;
            }

            // check for overlaps with existing notes
            let find_result: Vec<_> = lapper.find(curr_freq as usize, curr_freq as usize).collect();
            if !find_result.is_empty() {
                assert!(find_result.len() == 1);
                final_clips.push(combotones[find_result[0].val].clone());
                continue;
            }

            // finally, create the pitch shifted note
            let multiplier = curr_freq / freq;
            final_clips.push(fft.pitch_shift(multiplier).ifft_to_audio_clip());
        }

        final_clips
    }

    /// creates an interval based on the frequency that only overlaps with notes in the same chromatic note
    fn interval(freq: Sample, val: usize) -> Interval<usize, usize> {
        // chromatic notes differ in frequency by a multiple of 2^(1/12)
        // to prevent two chromatic notes from overlapping in intervals, we take another square root of the multiplier
        // and subtract by a small constant to account for precision errors
        let coefficient: Sample = Sample::from(2.0).powf(1.0 / 12.0).powf(0.5) - 0.005;

        let start = (freq / coefficient) as usize;
        let stop = (freq * coefficient) as usize;

        Interval { start, stop, val }
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
    fn test_combotones_intervals() {
        let source = path::Path::new("test_audio_clips/comboTones.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let combotones = TetrisClips::split_combotones(&clip);
        let combotones_fft = combotones.iter().map(|clip| clip.fft()).collect_vec();

        let frequencies = combotones_fft.iter().map(|fft| fft.most_significant_frequency()).collect_vec();
        let intervals = frequencies
            .iter()
            .map(|&f| TetrisClips::interval(f, 0))
            .collect_vec();

        let lapper = Lapper::new(intervals);
        for freq in frequencies.iter().map(|&f| f as usize) {
            let find_result: Vec<_> = lapper.find(freq, freq).collect();
            assert_eq!(find_result.len(), 1, "No two notes should overlap, each note should be unique");
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