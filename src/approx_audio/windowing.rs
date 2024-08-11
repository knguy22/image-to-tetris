use super::audio_clip::{AudioClip, Sample, Channel};
use std::f32::consts::PI;

impl AudioClip {
    // takes a window of the audio clip
    // pads the window with 0s if the window extends out of bounds
    #[allow(clippy::cast_precision_loss)]
    pub fn window(&self, start: usize, end: usize, windowing_fn: fn(&mut Channel)) -> Self {
        let mut channels = Vec::new();
        for channel in &self.channels {
            let end_in_range = std::cmp::min(end, channel.len());
            let mut to_push = channel[start..end_in_range].to_vec();
            to_push.resize(end - start, 0.0);
            windowing_fn(&mut to_push);
            channels.push(to_push);
        }
        let file_name = format!("{}_{}_{}.wav", self.file_name, start, end);
        Self {
            channels,
            file_name,
            duration: (end - start) as f64 / self.sample_rate,
            sample_rate: self.sample_rate,
            max_amplitude: self.max_amplitude,
            num_channels: self.num_channels,
            num_samples: end - start,
        }
    }
}

#[allow(unused)]
pub fn rectangle_window(_channel: &mut Channel) {
}

#[allow(clippy::cast_precision_loss, unused)]
pub fn hanning_window(channel: &mut Channel) {
    let big_n = channel.len() as Sample;
    for (n, sample) in channel.iter_mut().enumerate() {
        *sample *= 0.5 * (1.0 - (2.0 * PI * n as Sample / (big_n - 1.0)).cos());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let amplitude = 0.5;

        let start: usize = 1000;
        let end: usize = 8000;
        let window_len = end - start;

        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude, 1);
        let window_clip = clip.window(start, end, rectangle_window);

        assert!(window_clip.num_channels > 0);
        assert_eq!(window_clip.num_samples, window_len);
        assert_eq!(window_clip.channels[0].len(), window_len);
        assert!(window_clip.channels[0].iter().all(|v| *v == amplitude));
    }

    #[test]
    fn test_window_overflow() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let amplitude = 0.5;
        let num_samples = sample_rate as usize * duration as usize;

        let start: usize = 44000;
        let end: usize = 46000;
        let window_len = end - start;

        let clip = AudioClip::new_monotone(sample_rate, duration, amplitude, 1);
        let window_clip = clip.window(start, end, rectangle_window);

        assert!(window_clip.num_channels > 0);
        assert_eq!(window_clip.num_samples, window_len);
        assert_eq!(window_clip.channels[0].len(), window_len);

        // these samples should still be in range
        assert!(window_clip.channels[0].iter().take(num_samples - start).all(|v| *v == amplitude));

        // these samples should be out of range
        assert!(window_clip.channels[0].iter().skip(num_samples - start).all(|v| *v == 0.0));
    }

}