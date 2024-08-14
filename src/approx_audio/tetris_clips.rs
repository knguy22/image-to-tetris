use super::audio_clip::{AudioClip, Sample};

use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;
use rust_lapper::{Lapper, Interval};

type IntervalType = Interval<usize, usize>;

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

    fn extrapolate_chromatic_notes(combotones: &Vec<AudioClip>) {
        const MIN_FREQ: Sample = 50.0;
        const MAX_FREQ: Sample = 2000.0;

        let combotones_intervals: Vec<IntervalType> = combotones
            .iter()
            .map(|clip| Self::get_interval(clip.fft().most_significant_frequency()))
            .collect();
        let mut lapper = Lapper::new(combotones_intervals);
    }

    /// creates an interval based on the frequency that only overlaps with notes in the same chromatic note
    fn get_interval(freq: Sample) -> IntervalType {
        // chromatic notes differ in frequency by a multiple of 2^(1/12)
        // to prevent two chromatic notes from overlapping in intervals, we take another square root of the multiplier
        // and subtract by a small constant to account for precision errors
        let coefficient: Sample = Sample::from(2.0).powf(1.0 / 12.0) - 0.005;

        let start = (freq / coefficient) as usize;
        let stop = (freq * coefficient) as usize;

        Interval { start, stop, val: 0 }
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
            .map(|&f| TetrisClips::get_interval(f))
            .collect_vec();

        let lapper = Lapper::new(intervals);
        for freq in frequencies.iter().map(|&f| f as usize) {
            let find_result: Vec<_> = lapper.find(freq, freq).collect();
            assert_eq!(find_result.len(), 1, "No two notes should overlap, each note should be unique");
        }
    }
}