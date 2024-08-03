use itertools::Itertools;

use super::audio_clip::{AudioClip, Sample};

impl AudioClip {
    // gives a vector of sample indices that are onsets
    // this currently uses spectrum onset detection
    pub fn detect_onsets(&self) -> Vec<bool> {
        // perform short time fourier transform
        let stft = self.stft(None, None);
        
        // do feature processing
        // we only want the absolute values, not complex
        // we also take the gamma function to enhance the lower frequencies
        let gamma: f32 = 100.0;
        let stft = stft
            .iter()
            .map(|fft_result| {
                fft_result
                    .samples
                    .iter()
                    .map(|sample| (1.0 + gamma * sample.norm()).ln())
                    .collect_vec()
            })
            .collect_vec();

        // find the diffs (f32) between each adjacent fft result
        // also don't keep any negative values
        // and prepend a zero vector to keep the length consistent with the stft
        let mut diffs = stft
            .iter()
            .tuple_windows()
            .map(|(a, b)| {
                b
                    .iter()
                    .zip(a.iter())
                    .map(|(b, a)| if b - a >= 0.0 { b - a } else { 0.0 })
                    .reduce(|a, b| a + b)
                    .unwrap()
            })
            .collect_vec();
        diffs.insert(0, 0.0);

        // normalize the diffs
        let max_diff = diffs
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        diffs = diffs
            .iter()
            .map(|diff| diff / max_diff)
            .collect_vec();

        // perform onset detection
        let avg_diff = diffs
            .iter()
            .sum::<f32>()
            / diffs.len() as f32;
        let mut onsets = Vec::new();
        for diff in diffs.iter() {
            if *diff > avg_diff {
                onsets.push(true);
            }
            else {
                onsets.push(false);
            }
        }
        onsets
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path;

    #[test]
    fn test_onsets() {
        let clip = AudioClip::new(&path::PathBuf::from("test_audio_clips/comboTones.mp3")).unwrap();
        let onsets = clip.detect_onsets();
        let onset_count = onsets
            .iter()
            .filter(|&&b| b)
            .count();

        assert!(onset_count > 0);
        assert!(onset_count < clip.num_samples);
    }
}