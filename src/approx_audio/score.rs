use super::audio_clip::{AudioClip, Sample};

impl AudioClip {
    #[allow(clippy::cast_precision_loss, unused)]
    pub fn mse(&self, other: &Self) -> f64 {
        assert!(self.num_channels == other.num_channels);
        assert!((self.sample_rate - other.sample_rate).abs() < f64::EPSILON);

        let mut diff: f64 = 0.0;
        let zero: Sample = 0.0;
        for channel_idx in 0..self.num_channels {
            for sample_idx in 0..self.num_samples {
                let curr_sample = *self.channels[channel_idx].get(sample_idx).unwrap_or(&zero);
                let other_sample = *other.channels[channel_idx].get(sample_idx).unwrap_or(&zero);
                diff += (f64::from(curr_sample) - f64::from(other_sample)).powf(2.0);
            }
        }

        assert!(!diff.is_nan());
        diff / ((self.num_samples * self.num_channels) as f64)
    }

    #[allow(clippy::cast_precision_loss, unused)]
    pub fn dot_product(&self, other: &Self) -> f64 {
        assert!(self.num_channels == other.num_channels);
        assert!((self.sample_rate - other.sample_rate).abs() < f64::EPSILON);

        let mut diff: f64 = 0.0;
        let zero: Sample = 0.0;
        for channel_idx in 0..self.num_channels {
            for sample_idx in 0..self.num_samples {
                let curr_sample = *self.channels[channel_idx].get(sample_idx).unwrap_or(&zero);
                let other_sample = *other.channels[channel_idx].get(sample_idx).unwrap_or(&zero);
                diff += f64::from(curr_sample) * f64::from(other_sample);
            }
        }

        assert!(!diff.is_nan());
        diff / ((self.num_samples * self.num_channels) as f64)
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, fs};
    use super::*;
    use crate::approx_audio::resample;

    #[test]
    fn test_mse_same_file() {
        let source = Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let diff = clip.diff(&clip);
        assert_eq!(diff, 0.0);
    }

    #[test]
    fn test_mse_different_files() {
        // first resample the audio clips to 44100 Hz
        let sample_rate = 44100.0;
        let source_dir = Path::new("test_audio_clips");
        let resample_source_dir = Path::new("test_resampled_audio_clips");
        resample::run_dir(source_dir, resample_source_dir, sample_rate).expect("failed to resample audio clips");

        // the same file should have the lowest diff with itself
        let mut clips = Vec::new();
        for source in resample_source_dir.read_dir().unwrap() {
            clips.push(AudioClip::new(&source.unwrap().path()).expect("failed to create audio clip"));
        }

        let self_diff = clips[0].diff(&clips[0]);
        assert!(clips.iter().skip(1).all(|clip| clip.diff(&clips[0]) > self_diff));

        // cleanup
        fs::remove_dir_all(resample_source_dir).expect("failed to remove resampled audio clips");
    }

}