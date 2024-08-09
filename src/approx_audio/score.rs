use super::audio_clip::{AudioClip, Sample};
use std::iter::Iterator;
use itertools::Itertools;

impl AudioClip {
    #[allow(clippy::cast_precision_loss, unused)]
    pub fn mse(&self, other: &Self) -> f64 {
        let diff = self.zip_samples(other)
            .map(|(s1, s2)| (f64::from(s1) - f64::from(s2)).powf(2.0))
            .sum::<f64>();
        assert!(!diff.is_nan());
        diff / ((self.num_samples * self.num_channels) as f64)
    }

    #[allow(clippy::cast_precision_loss, unused)]
    pub fn dot_product(&self, other: &Self) -> f64 {
        let diff = self.zip_samples(other)
            .map(|(s1, s2)| f64::from(s1) * f64::from(s2))
            .sum::<f64>();
        assert!(!diff.is_nan());
        diff / ((self.num_samples * self.num_channels) as f64)
    }

    fn zip_samples<'a>(&'a self, other: &'a Self) -> impl Iterator<Item = (Sample, Sample)> + 'a {
        assert!(self.num_channels == other.num_channels);
        assert!((self.sample_rate - other.sample_rate).abs() < f64::EPSILON);
        self.channels.iter().zip_eq(other.channels.iter()).flat_map(|(channel_self, channel_other)|
            channel_self.iter().zip_longest(channel_other.iter())
                .map(|pair| match pair {
                    itertools::EitherOrBoth::Both(s1, s2) => (*s1, *s2),
                    itertools::EitherOrBoth::Left(s1) => (*s1, 0.0),
                    itertools::EitherOrBoth::Right(s2) => (0.0, *s2),
                }
            )
        )
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
        let diff = clip.mse(&clip);
        assert_eq!(diff, 0.0);
    }

    #[test]
    fn test_dot_same_file() {
        let source = Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let diff = clip.dot_product(&clip);
        assert_ne!(diff, 0.0);
    }

    #[test]
    fn test_different_files() {
        let clips = get_clips();

        // mse
        let self_diff = clips[0].mse(&clips[0]);
        assert!(clips.iter().skip(1).all(|clip| clip.mse(&clips[0]) > self_diff), "Similar files should have lower MSEs");

        // dot 
        let self_diff = clips[0].dot_product(&clips[0]);
        assert!(clips.iter().skip(1).all(|clip| clip.dot_product(&clips[0]) < self_diff), "Similar files should have greater dot products");

        cleanup_clips();
    }

    fn get_clips() -> Vec<AudioClip> {
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

        clips
    }

    fn cleanup_clips() {
        let resample_source_dir = Path::new("test_resampled_audio_clips");
        fs::remove_dir_all(resample_source_dir).expect("failed to remove resampled audio clips");
    }

}