use super::audio_clip::{AudioClip, Sample};
use super::fft::FFTResult;
use super::pitch::CHROMATIC_MULTIPLIER;

use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;
use rust_lapper::{Lapper, Interval};

const INVALID_CLIP_ID: i32 = -1;

#[derive(Debug)]
pub struct TetrisClips {
    pub clips: Vec<AudioClip>,
    lapper: Lapper<usize, i32>,
}

impl TetrisClips {
    pub fn new(source: &Path) -> Result<TetrisClips> {
        let tetris_clips = TetrisClips { clips: Vec::new(), lapper: Lapper::new(Vec::new()) };

        for path in source.read_dir()? {
            let path = path?;
            let clip = AudioClip::new(&path.path())?;

            match path.file_name() {
                // combotones are made of multiple clips, not just one
                name if name == "comboTones.mp3" || name == "comboTones.wav" => {
                    let combotones = TetrisClips::split_combotones(&clip);
                },
                _ => (),
            }
        }

        Ok(tetris_clips)
    }

    #[allow(clippy::cast_precision_loss)]
    fn split_combotones(clips: &AudioClip) -> Vec<AudioClip> {
        const NUM_COMBOS: usize = 15;
        let combo_duration = clips.duration / NUM_COMBOS as f64;

        // there may be an extra combo due to rounding errors; drop it 
        let combos = clips.split_by_duration(combo_duration);
        combos.into_iter().take(NUM_COMBOS).collect()
    }

    fn push_raw_combotones(&mut self, clips: &Vec<AudioClip>) {
        for (curr, next) in clips.iter().tuple_windows() {
            // regardless of the result, we push the current combotone
            self.clips.push(curr.clone());
            let curr_id = (self.clips.len() - 1) as i32;

            // combotones are guaranted to be in ascending pitch order
            // combotones are a major scale, so they do not guarantee having all chromatic notes
            // this means that any gaps in chromatic notes are at most 1 note long
            let curr_fundamental = curr.fft().most_significant_frequency();
            let next_fundamental = next.fft().most_significant_frequency();

            let next_is_subsequent_chromatic: bool = next_fundamental / curr_fundamental < CHROMATIC_MULTIPLIER + 0.02;
            if next_is_subsequent_chromatic {
                let interval = Interval {start: curr_fundamental as usize, stop: next_fundamental as usize, val: curr_id};
                self.lapper.insert(interval);
            } else {
                // in this case, push two intervals as there is only one chromatic note in between
                // the second one doesn't have any clip associated with it yet
                let estimated_fundamental = curr_fundamental * CHROMATIC_MULTIPLIER;
                let first_interval = Interval {start: curr_fundamental as usize, stop: estimated_fundamental as usize, val: curr_id};
                let second_interval = Interval {start: estimated_fundamental as usize, stop: next_fundamental as usize, val: INVALID_CLIP_ID};

                self.lapper.insert(first_interval);
                self.lapper.insert(second_interval);
            }
        }

        // don't forget to push the last one
        let last = clips.last().unwrap();
        self.clips.push(last.clone());
        let last_id = (self.clips.len() - 1) as i32;

        let last_fundamental = last.fft().most_significant_frequency();
        let expected_fundamental = last_fundamental * CHROMATIC_MULTIPLIER;
        let interval = Interval {start: last_fundamental as usize, stop: expected_fundamental as usize, val: last_id};
        self.lapper.insert(interval);
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
    fn test_push_raw_combotones() {
        let source = path::Path::new("test_audio_clips/comboTones.mp3");
        let combotones = AudioClip::new(&source).expect("failed to create audio clip");
        let split_combotones = TetrisClips::split_combotones(&combotones);

        // create an empty tetris clips with no initialization
        let mut tetris_clips = TetrisClips {
            clips: Vec::new(),
            lapper: Lapper::new(Vec::new()),
        };
        tetris_clips.push_raw_combotones(&split_combotones);

        // check that there is only one valid interval for each possible frequency in between
        let first_fundamental = split_combotones.first().unwrap().fft().most_significant_frequency() as usize;
        let last_fundamental = split_combotones.last().unwrap().fft().most_significant_frequency();
        let last_valid_freq = (last_fundamental * CHROMATIC_MULTIPLIER) as usize;
        for freq in first_fundamental..last_valid_freq {
            let lapper_res = tetris_clips.lapper.find(freq, freq + 1).collect_vec();
            assert!(lapper_res.len() == 1);
        }
    }
}