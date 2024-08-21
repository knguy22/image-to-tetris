use super::audio_clip::{AudioClip, Sample};
use super::fft::FFTResult;
use super::pitch::CHROMATIC_MULTIPLIER;

use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;
use rust_lapper::{Lapper, Interval};

// using the frequencies here: https://en.wikipedia.org/wiki/Piano_key_frequencies
const MIN_FREQ: Sample = 65.40639; // C great octave
const MAX_FREQ: Sample = 1046.502; // C''' 3-line octave
const INVALID_CLIP_ID: usize = usize::MAX;

#[derive(Debug)]
pub struct TetrisClips {
    pub clips: Vec<TetrisClip>,
    lapper: Lapper<usize, usize>,
}

#[derive(Clone, Debug)]
pub struct TetrisClip {
    pub audio: AudioClip,
    pub fft: FFTResult,
}

impl TetrisClips {
    pub fn new(source: &Path) -> Result<TetrisClips> {
        let mut tetris_clips = TetrisClips { clips: Vec::new(), lapper: Lapper::new(Vec::new()) };

        for path in source.read_dir()? {
            let path = path?;
            let clip = AudioClip::new(&path.path())?;

            match path.file_name() {
                // combotones are made of multiple clips, not just one
                name if name == "comboTones.mp3" || name == "comboTones.wav" => {
                    let combotones = TetrisClips::split_combotones(&clip);
                    let mut skipped = tetris_clips.push_raw_combotones(&combotones);
                    skipped.extend(tetris_clips.compute_pitch_shifted_intervals());
                    tetris_clips.populate_skipped_intervals(&skipped);
                },
                _ => (),
            }
        }

        Ok(tetris_clips)
    }

    pub fn get_combotone(&self, freq: usize) -> Option<(&TetrisClip, Interval<usize, usize>)> {
        let freq_interval = Interval {start: freq, stop: freq + 1, val: 0};
        let res = self.lapper.find(freq_interval.start, freq_interval.stop).collect_vec();
        assert!(res.len() <= 1, "found more than one note at frequency {freq}, intervals: {res:?}");

        if res.len() == 1 {
            return Some((&self.clips[res[0].val], res[0].clone()))
        }
        None
    }

    #[allow(clippy::cast_precision_loss)]
    fn split_combotones(clips: &AudioClip) -> Vec<AudioClip> {
        const NUM_COMBOS: usize = 15;
        let combo_duration = clips.duration / NUM_COMBOS as f64;

        // there may be an extra combo due to rounding errors; drop it 
        let combos = clips.split_by_duration(combo_duration);
        combos.into_iter().take(NUM_COMBOS).collect()
    }

    /// takes a list of clips and inserts their clips + intervals if appropriate
    /// chromatic notes skipped will have their estimated intervals returned for later use
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
    fn push_raw_combotones(&mut self, clips: &[AudioClip]) -> Vec<Interval<usize, usize>> {
        let mut skipped_intervals = Vec::new();

        for (curr, next) in clips.iter().tuple_windows() {
            let curr_fft = curr.fft();
            let next_fft = next.fft();
            let curr_fundamental = curr_fft.most_significant_frequency();
            let next_fundamental = next_fft.most_significant_frequency();

            // regardless of the result, we push the current combotone
            self.clips.push(TetrisClip { audio: curr.clone(), fft: curr_fft });
            let curr_id = self.clips.len() - 1;

            // combotones are guaranted to be in ascending pitch order
            // combotones are a major scale, so they do not guarantee having all chromatic notes
            // this means that any gaps in chromatic notes are at most 1 note long
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
                skipped_intervals.push(second_interval);
            }
        }

        // don't forget to push the last one
        let last = clips.last().unwrap();
        let last_fft = last.fft();
        let last_fundamental = last_fft.most_significant_frequency();
        self.clips.push(TetrisClip { audio: last.clone(), fft: last_fft });
        let last_id = self.clips.len() - 1;

        let expected_fundamental = last_fundamental * CHROMATIC_MULTIPLIER;
        let interval = Interval {start: last_fundamental as usize, stop: expected_fundamental as usize, val: last_id};
        self.lapper.insert(interval);

        skipped_intervals
    }

    /// this should be run after `push_raw_combotones` so there are some intervals in play
    /// this also returns intervals that will be pitch shifted for later use
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
    fn compute_pitch_shifted_intervals(&self) -> Vec<Interval<usize, usize>> {
        assert!(!self.lapper.is_empty());
        let mut intervals = Vec::new();

        // the min freq can be obtained from the lowest fundamental
        // however, the max freq must be obtained from the highest freq included from the last combotone's fundamental
        let mut curr_min_freq = self.clips.first().unwrap().fft.most_significant_frequency();
        let mut curr_max_freq = self.clips.last().unwrap().fft.most_significant_frequency() * CHROMATIC_MULTIPLIER;

        // extrapolate intervals downward first
        while curr_min_freq > MIN_FREQ {
            let next_min_freq = (curr_min_freq / CHROMATIC_MULTIPLIER) as usize;
            let interval = Interval {start: next_min_freq, stop: curr_min_freq as usize, val: INVALID_CLIP_ID};
            intervals.push(interval);

            // account for rounding errors
            curr_min_freq = next_min_freq as Sample;
            assert!(curr_min_freq as usize == next_min_freq);
        }

        // then extrapolate intervals upward
        while curr_max_freq < MAX_FREQ {
            let next_max_freq = (curr_max_freq * CHROMATIC_MULTIPLIER) as usize;
            let interval = Interval {start: curr_max_freq as usize, stop: next_max_freq, val: INVALID_CLIP_ID};
            intervals.push(interval);

            // account for rounding errors
            curr_max_freq = next_max_freq as Sample;
            assert!(curr_max_freq as usize == next_max_freq);
        }

        intervals
    }

    /// creates corresponding pitch-shifted audio clips for skipped intervals and pushes them to self.clips
    #[allow(clippy::cast_precision_loss)]
    fn populate_skipped_intervals(&mut self, intervals: &[Interval<usize, usize>]) {
        // create iterators to loop through existing combotones so pitch shifted audio clips aren't all the same
        let combotones: Vec<TetrisClip> = self.clips.iter().take(7).cloned().collect();
        let combotones_iter = combotones.iter().cycle();

        for (interval, combotone) in intervals.iter().zip(combotones_iter) {
            let target_fundamental = interval.start as Sample;
            let curr_fundamental = combotone.fft.most_significant_frequency();
            let multiplier = target_fundamental / curr_fundamental;
            let pitch_shifted = combotone.fft.pitch_shift(multiplier);
            self.clips.push(TetrisClip { audio: pitch_shifted.ifft_to_audio_clip(), fft: pitch_shifted});

            let clip_id = self.clips.len() - 1;
            let new_interval = Interval {val: clip_id, ..*interval};
            self.lapper.insert(new_interval);
        }
    }

    #[allow(dead_code)]
    pub fn dump(&self, output_dir: &Path) -> Result<()> {
        for clip in &self.clips {
            let clip_path = PathBuf::from(clip.audio.file_name.clone());
            let clip_file_name = clip_path.file_name().unwrap();
            clip.audio.write(Some(&output_dir.join(clip_file_name)))?;
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

        for clip in tetris_clips.clips.iter().map(|clip| &clip.audio) {
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
        let skipped = tetris_clips.push_raw_combotones(&split_combotones);
        
        // combotones contains 2 octaves
        assert!(skipped.len() + split_combotones.len() == 25);
        for interval in skipped {
            tetris_clips.lapper.insert(interval);
        }

        // check that there is only one valid interval for each possible frequency in between
        let first_fundamental = split_combotones.first().unwrap().fft().most_significant_frequency() as usize;
        let last_fundamental = split_combotones.last().unwrap().fft().most_significant_frequency();
        let last_valid_freq = (last_fundamental * CHROMATIC_MULTIPLIER) as usize;
        for freq in first_fundamental..last_valid_freq {
            let lapper_res = tetris_clips.lapper.find(freq, freq + 1).collect_vec();
            assert!(lapper_res.len() == 1);
        }
    }

    #[test]
    #[ignore]
    fn test_all_combotones() {
        let source = path::Path::new("test_audio_clips/comboTones.mp3");
        let combotones = AudioClip::new(&source).expect("failed to create audio clip");
        let split_combotones = TetrisClips::split_combotones(&combotones);

        // create an empty tetris clips with no initialization
        let mut tetris_clips = TetrisClips {
            clips: Vec::new(),
            lapper: Lapper::new(Vec::new()),
        };
        let mut skipped = tetris_clips.push_raw_combotones(&split_combotones);
        skipped.extend(tetris_clips.compute_pitch_shifted_intervals());
        tetris_clips.populate_skipped_intervals(&skipped);

        let min_freq = MIN_FREQ as usize;
        let max_freq = MAX_FREQ as usize;
        for freq in min_freq..max_freq {
            let lapper_res = tetris_clips.lapper.find(freq, freq + 1).collect_vec();

            assert!(lapper_res.len() == 1);
            assert!(lapper_res[0].val != INVALID_CLIP_ID);
            assert!(tetris_clips.clips[lapper_res[0].val as usize].audio.num_samples > 0);
        }

        // save the combotones
        let output = path::Path::new("test_chromatic_tones.wav");
        let mut output_clip = tetris_clips.clips[0].audio.clone();
        for clip in tetris_clips.clips.iter().skip(1) {
            output_clip.append_mut(&clip.audio);
        }
        output_clip.write(Some(output)).expect("failed to save combotones to wav");
    }
}