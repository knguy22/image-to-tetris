use crate::approx_audio::pitch::NoteTracker;

use super::audio_clip::{AudioClip, Sample};
use super::fft::FFTResult;

use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;

#[derive(Debug)]
pub struct TetrisClips {
    pub clips: Vec<AudioClip>,
    pub note_tracker: NoteTracker,
}

impl TetrisClips {
    pub fn new(source: &Path) -> Result<TetrisClips> {
        let mut tetris_clips = TetrisClips { clips: Vec::new(), note_tracker: NoteTracker::new() };

        for path in source.read_dir()? {
            let path = path?;
            let clip = AudioClip::new(&path.path())?;

            match path.file_name() {
                // combotones are made of multiple clips, not just one
                name if name == "comboTones.mp3" || name == "comboTones.wav" => {
                    let combotones = TetrisClips::split_combotones(&clip);
                    tetris_clips.extrapolate_chromatic_notes(&combotones);
                },
                _ => (),
            }
        }

        Ok(tetris_clips)
    }

    pub fn get_combotone(&self, freq: Sample) -> Option<&AudioClip> {
        let id = self.note_tracker.get_note(freq);
        match id {
            Some(id) => Some(&self.clips[id]),
            None => None,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn split_combotones(clips: &AudioClip) -> Vec<AudioClip> {
        const NUM_COMBOS: usize = 15;
        let combo_duration = clips.duration / NUM_COMBOS as f64;

        // there may be an extra combo due to rounding errors; drop it 
        let combos = clips.split_by_duration(combo_duration);
        combos.into_iter().take(NUM_COMBOS).collect()
    }

    fn extrapolate_chromatic_notes(&mut self, combotones: &Vec<AudioClip>) {
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
        self.clips.extend(combotones.clone());
        for (i, &freq) in combotones_freq.iter().enumerate() {
            self.note_tracker.add_note(freq, i).expect("failed to add note");
        }

        // set up a target audio magnitude to adjust for pitch shifting losing energy
        let target_max_amplitude = combotones
            .iter()
            .map(|clip| clip.max_amplitude)
            .fold(f32::from(0.0), |a, b| a.max(b))
            / 2.0;

        let mut curr_freq = combotones_freq[0] / (Sample::from(2.0).powf(2.0));
        loop {
            let (&freq, fft) = lower_notes_iter.next().unwrap();

            // limit check
            curr_freq *= chromatic_diff;
            if curr_freq > MAX_FREQ {
                break;
            }
            if self.note_tracker.get_note(curr_freq).is_some() {
                continue;
            }

            // finally, create the pitch shifted note; make sure to scale the amplitude
            let pitch_multiplier = curr_freq / freq;
            let to_push = fft.pitch_shift(pitch_multiplier).ifft_to_audio_clip();
            let magnitude_multiplier = target_max_amplitude / to_push.max_amplitude;
            let to_push = to_push.scale_amplitude(magnitude_multiplier);

            self.clips.push(to_push);
            self.note_tracker.add_note(curr_freq, self.clips.len() - 1).expect("failed to add note");
        }
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
            note_tracker.add_note(freq, 0).expect("failed to add note"); 
        }

        for &freq in &frequencies {
            assert!(note_tracker.get_note(freq).is_some());
        }
    }

    #[test]
    fn test_tetris_clips_pitch_shifted() {
        let source = path::Path::new("test_audio_clips");
        let tetris_clips = TetrisClips::new(&source).expect("failed to create tetris clips");

        let clips_fft = tetris_clips.clips.iter().map(|clip| clip.fft()).collect_vec();
        let frequencies = clips_fft.iter().map(|fft| fft.most_significant_frequency()).collect_vec();

        for &freq in &frequencies {
            let combotone = tetris_clips.get_combotone(freq).unwrap();
            let fundamental_freq = combotone.fft().most_significant_frequency() as usize;
            let expected_interval = NoteTracker::interval(freq, 0);
            println!("freq: {freq}, fundamental_freq: {fundamental_freq}, expected_interval: {expected_interval:?}");

            assert!(expected_interval.start <= fundamental_freq, "interval: {expected_interval:?}, fundamental_freq: {fundamental_freq}");
            assert!(expected_interval.stop >= fundamental_freq, "interval: {expected_interval:?}, fundamental_freq: {fundamental_freq}");
        }
    }
}