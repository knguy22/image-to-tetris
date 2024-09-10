use crate::utils::progress_bar;
use super::{audio_clip::{AudioClip, Channel, Sample}, windowing::rectangle_window};
use std::fmt;
use std::path::Path;

use anyhow::Result;
use itertools::Itertools;
use median::Filter;
use rustfft::{FftPlanner, num_complex::Complex};
use rayon::prelude::*;

pub type FFTSample = Complex<Sample>;
pub type FFTChannel = Vec<FFTSample>;
pub type STFT = Vec<FFTResult>;

/// a channel of norms; usually converted from a channel of complex samples
pub type FFTChannelNorm = Vec<Sample>;

/// multiple `FFTChannelNorms` over different channels
/// 
/// indexed by channel,sample
pub type FFTNorms = Vec<FFTChannelNorm>;

/// multiple `FFTNorms` over different timestamps
/// 
/// indexed by timestamp,channel,sample
pub type STFTNorms = Vec<FFTNorms>;

#[derive(Clone)]
pub struct FFTResult {
    pub channels: Vec<FFTChannel>,
    pub frequency_resolution: f64,
    pub sample_rate: f64,
    pub num_samples: usize,
}

impl AudioClip {
    /// performs a short time fourier transform on the audio clip
    /// `window_size` is the number of samples in the window
    /// `hop_size` is the number of samples between each window
    pub fn stft(&self, window_size: usize, hop_size: usize, windowing_fn: fn(&mut Channel)) -> STFT {
        assert!(hop_size > 0, "hop size must be positive");

        let mut stft_res = Vec::new();
        for curr_index in (0..self.num_samples).step_by(hop_size) {
            let window = self.window(curr_index, curr_index + window_size, windowing_fn);
            stft_res.push(window.fft());
        }

        stft_res
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn fft(&self) -> FFTResult {
        assert!(self.channels.iter().all(|c| c.len() == self.num_samples));

        let mut planner = FftPlanner::<Sample>::new();
        let fft = planner.plan_fft_forward(self.num_samples);

        let mut complex_channels = self.channels
            .iter()
            .map(|channel| {
                channel
                    .iter()
                    .map(|&sample| FFTSample::new(sample, 0.0))
                    .collect_vec()
            })
            .collect_vec();

        // perform the FFT for each each channel
        for channel in &mut complex_channels {
            fft.process(channel);
        }

        FFTResult {
            channels: complex_channels,
            frequency_resolution: self.sample_rate / self.num_samples as f64,
            sample_rate: self.sample_rate,
            num_samples: self.num_samples,
        }
    }
}

pub fn get_norms(stft: &[FFTResult]) -> STFTNorms {
    fn norms_fft_result(fft_result: &FFTResult) -> FFTNorms {
        fft_result
            .channels
            .iter()
            .map(|channel| channel.iter().map(|&sample| sample.norm()).collect_vec())
            .collect_vec()
    }

    stft
        .iter()
        .map(norms_fft_result)
        .collect_vec()
}

/// separate the harmonic from the percussive component;
/// returns (harmonic,percussive)
/// this whole procedure was implemented using https://www.audiolabs-erlangen.de/resources/MIR/FMP/C8/C8S1_HPS.html as a reference
pub fn separate_harmonic_percussion(clip: &AudioClip, window_size: usize, hop_size: usize) -> (AudioClip, AudioClip) {
    assert!(hop_size > 0, "hop size must be positive");

    // Step 1: Use STFT, but don't use any overlapping
    let stft = clip.stft(window_size, hop_size, rectangle_window);

    // Step 2: Use Norms to transpose from complex values
    let norms = get_norms(&stft);

    // Step 3: Apply median filterings horizontally and vertically to distinguish harmonics and percussion respectively
    // These two constants are chosen arbitrarily to help determine the window sizes
    let length_sec: f64 = 0.1;
    let window_h = make_odd((length_sec * clip.sample_rate / hop_size as f64).ceil() as usize);
    let filt_h = medfilt_h(&norms, window_h);

    let length_hz: f64 = 100.0;
    let window_v = make_odd((length_hz * window_size as f64 / clip.sample_rate) as usize);
    let filt_v = medfilt_v(&norms, window_v);

    // Step 4: Transform the filters into masks
    let (mask_h, mask_v) = binary_mask(&filt_h, &filt_v);

    // Step 5: Apply the masks to the original STFT to create two final STFTS
    let num_timestamps = mask_h.len();
    let num_channels = mask_h[0].len();
    let num_bins = mask_h[0][0].len();
    let mut stft_h = stft.clone();
    let mut stft_v = stft.clone();

    for timestamp in 0..num_timestamps {
        for channel in 0..num_channels {
            for bin in 0..num_bins {
                // apply the binary filter
                stft_h[timestamp].channels[channel][bin] *= mask_h[timestamp][channel][bin];
                stft_v[timestamp].channels[channel][bin] *= mask_v[timestamp][channel][bin];
            }
        }
    }

    // Step 6: Use ISTFT to create two new audio clips
    (inverse_stft(&stft_h, hop_size, rectangle_window), inverse_stft(&stft_v, hop_size, rectangle_window))
}

fn make_odd(num: usize) -> usize {
    if num % 2 == 0 {
        num + 1
    } else {
        num
    }
}

/// inverse short time fourier transform
/// window size is implicitly given by the stft
pub fn inverse_stft(stft: &STFT, hop_size: usize, windowing_fn: fn(&mut Channel)) -> AudioClip {
    // get all audio clips and perform the windowing function
    let clips: Vec<_> = stft
        .par_iter()
        .map(|fftres| fftres.ifft_to_audio_clip())
        .map(|clip| clip.window(0, clip.num_samples, windowing_fn))
        .collect();

    // gather important information
    let first = &clips[0];
    let window_size = first.num_samples;
    let num_samples = hop_size * clips.len();

    // then perform the overlapping using the hop size as a guide
    let mut final_clip = AudioClip::new_monoamplitude(first.sample_rate, num_samples, 0.0, first.num_channels);
    for (clip, start) in clips.iter().zip((0..num_samples).step_by(hop_size)) {
        for channel in 0..first.num_channels {
            for sample in start..(start + window_size).min(num_samples) {
                final_clip.channels[channel][sample] += clip.channels[channel][sample - start];
            }
        }
    }

    final_clip
}

pub fn binary_mask(input_h: &STFTNorms, input_v: &STFTNorms) -> (Vec<Vec<Vec<Sample>>>, Vec<Vec<Vec<Sample>>>) {
    assert_eq!(input_v.len(), input_h.len(), "dimensions not the same");
    assert_eq!(input_v[0].len(), input_h[0].len(), "dimensions not the same");
    assert_eq!(input_v[0][0].len(), input_h[0][0].len(), "dimensions not the same");

    let num_timestamps = input_h.len();
    let num_channels = input_h[0].len();
    let num_bins = input_h[0][0].len();

    let mut output_h = vec![vec![vec![0.0; num_bins]; num_channels]; num_timestamps];
    let mut output_v = vec![vec![vec![0.0; num_bins]; num_channels]; num_timestamps];

    // iterate through each combination
    for timestamp in 0..num_timestamps {
        for channel in 0..num_channels {
            for bin in 0..num_bins {
                // create the binary filter
                let choose_h: bool = input_h[timestamp][channel][bin] >= input_v[timestamp][channel][bin];
                output_h[timestamp][channel][bin] = if choose_h {1.0} else {0.0};
                output_v[timestamp][channel][bin] = if choose_h {0.0} else {1.0};
            }
        }
    }

    (output_h, output_v)
}

/// performs a median filter across the vertical axis, which is the frequency axis
pub fn medfilt_v(stft_norms: &STFTNorms, window_size: usize) -> STFTNorms {
    assert!(window_size % 2 == 1, "window_size must be odd");

    let pb = progress_bar(stft_norms.len()).unwrap();
    pb.set_message("Median filtering vertical...");
    let filtered = stft_norms
        .par_iter()
        .map(|fft_result| {
            pb.inc(1);
            fft_result
                .iter()
                .map(|channel| medfilt_slice(channel, window_size))
                .collect()
        })
        .collect();
    pb.finish_with_message("Done median filtering vertical!");

    filtered
}

/// performs a median filter across the horizontal axis, which is the time axis
pub fn medfilt_h(stft_norms: &STFTNorms, window_size: usize) -> STFTNorms {
    assert!(window_size % 2 == 1, "window_size must be odd");

    // STFTNorms indexed time, channel, bin
    let num_timestamps = stft_norms.len();
    let num_channels = stft_norms[0].len();
    let num_bins = stft_norms[0][0].len();
    let pb = progress_bar(num_channels * num_bins).unwrap();
    pb.set_message("Median filtering horizontal...");

    // create the filtered vectors
    // indexed by channel, bin, time
    let med_filtered: Vec<Vec<Vec<Sample>>> = (0..num_channels)
        .into_iter()
        .map(|channel| {
            let channel_results = (0..num_bins)
                .into_par_iter()
                .map(|bin| {
                    let orig_vec = (0..num_timestamps)
                        .map(|timestamp| stft_norms[timestamp][channel][bin])
                        .collect_vec();
                    pb.inc(1);
                    medfilt_slice(&orig_vec, window_size)
                })
                .collect();
            channel_results
        })
        .collect();
    pb.finish_with_message("Done median filtering horizontal!");

    // reassemble the filtered vectors into an STFTNorm
    let mut final_stft = stft_norms.clone();
    for channel in 0..num_channels {
        for bin in 0..num_bins {
            for timestamp in 0..num_timestamps {
                final_stft[timestamp][channel][bin] = med_filtered[channel][bin][timestamp];
            }
        }
    }
    final_stft
}


fn medfilt_slice<T>(slice: &[T], window_size: usize) -> Vec<T> 
    where T: Copy + Clone + PartialOrd
{
    slice
        .iter()
        .scan(Filter::new(window_size), |filter, &val| Some(filter.consume(val)))
        .collect_vec()
}

impl FFTResult {
    #[allow(clippy::cast_precision_loss)]
    pub fn empty(sample_rate: f64, num_samples: usize, num_channels: usize) -> Self {
        FFTResult {
            channels: vec![vec![Complex::new(0.0, 0.0); num_samples]; num_channels],
            frequency_resolution: sample_rate / num_samples as f64,
            sample_rate,
            num_samples,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn ifft_to_audio_clip(&self) -> AudioClip {
        let channels = self.ifft();
        let num_channels = channels.len();
        let duration = self.num_samples as f64 / self.sample_rate;
        let max_amplitude = channels
            .iter()
            .map(|channel| 
                channel
                    .iter()
                    .fold(0.0, |a, &b| Sample::max(a, b))
            )
            .fold(0.0, Sample::max) ;
        assert!(max_amplitude.is_finite());

        AudioClip {
            channels,
            file_name: String::new(),
            duration,
            sample_rate: self.sample_rate,
            max_amplitude,
            num_channels,
            num_samples: self.num_samples,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn ifft(&self) -> Vec<Channel> {
        let mut planner = FftPlanner::<Sample>::new();
        let fft = planner.plan_fft_inverse(self.num_samples);

        self.channels
            .iter()
            .map(|channel| {
                let mut ifft_samples = channel.clone();
                fft.process(&mut ifft_samples);

                // amplitudes across iffts are not standardized so we need to normalize them (with sample len)
                ifft_samples.iter().map(|s| s.re / self.num_samples as Sample).collect()
            })
            .collect_vec()
    }

    #[allow(clippy::cast_precision_loss, dead_code)]
    pub fn dump(&self, output: &Path) -> Result<()> {
        let mut wtr = csv::Writer::from_path(output)?;
        wtr.write_record(["channel", "frequency", "norm"])?;
        for (i, channel) in self.channels.iter().enumerate() {
            for (j, sample) in channel.iter().enumerate() {
                let frequency = self.frequency_resolution * j as f64;
                wtr.write_record(&[i.to_string(), frequency.to_string(), sample.norm().to_string()])?;
            }
        }

        Ok(())
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl fmt::Debug for FFTResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FFTResult")
            .field("frequency_resolution", &self.frequency_resolution)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::approx_audio::windowing::rectangle_window;

    #[test]
    fn test_fft() {
        let source = Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        assert!(fft.channels.iter().all(|c| c.len() == clip.num_samples));
    }

    #[test]
    fn test_fft_monoamplitude() {
        let sample_rate = 44100.0;
        let num_samples = 1000;
        let amplitude = 0.5;

        let clip = AudioClip::new_monoamplitude(sample_rate, num_samples, amplitude, 1);
        let fft = clip.fft();
        assert!(fft.channels.iter().all(|c| c.len() == clip.num_samples));
    }

    #[test]
    fn test_stft() {
        let sample_rate = 44100.0;
        let num_samples = 1000;
        let amplitude = 0.5;

        let window = 1024;
        let hop = window / 4;
        let clip = AudioClip::new_monoamplitude(sample_rate, num_samples, amplitude, 1);
        let stft = clip.stft(window, hop, rectangle_window);

        assert_eq!(stft.len(), clip.num_samples / hop + 1);
    }

    #[test]
    fn test_stft_2() {
        let sample_rate = 24100.0;
        let num_samples = 1000;
        let amplitude = 0.6;

        let window = 2048;
        let hop = window / 4;
        let clip = AudioClip::new_monoamplitude(sample_rate, num_samples, amplitude, 1);
        let stft = clip.stft(window, hop, rectangle_window);

        assert_eq!(stft.len(), clip.num_samples / hop + 1);
    }

    #[test]
    fn test_dump() {
        let source = Path::new("test_audio_clips/a6.mp3");
        let output = Path::new("test_fft.csv");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        fft.dump(output).unwrap();

        // cleanup
        std::fs::remove_file(&output).unwrap();
    }

    #[test]
    fn test_ifft() {
        let source = Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        let ifft = fft.ifft();
        assert_eq!(ifft.len(), clip.num_channels);
        assert!(ifft.iter().all(|c| c.len() == clip.num_samples));
    }

    #[test]
    fn test_ifft_clip() {
        let source = Path::new("test_audio_clips/a6.mp3");
        let output = Path::new("test_ifft_clip.wav");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");
        let fft = clip.fft();
        let ifft_clip = fft.ifft_to_audio_clip();

        assert!((clip.duration - ifft_clip.duration).abs() < 0.001);
        assert!(clip.sample_rate == ifft_clip.sample_rate);

        // there is no need to check max amplitude for exact correctness because the channels have been merged with the ifft, which means
        // amplitudes are averaged out
        assert!(ifft_clip.max_amplitude <= clip.max_amplitude);

        ifft_clip.write(Some(output)).unwrap();
    }

    #[test]
    fn test_istft(){
        let sample_rate = 24100.0;
        let num_samples = 4000;
        let amplitude = 0.6;

        let hop = 1000;
        let window = 1000;
        let clip = AudioClip::new_monoamplitude(sample_rate, num_samples, amplitude, 1);

        // hop = window since we want no overlapping
        let stft = clip.stft(window, hop, rectangle_window);
        let istft = inverse_stft(&stft, hop, rectangle_window);
        assert!((clip.duration - istft.duration).abs() < 0.001, "Duration: {} {}", clip.duration, istft.duration);
        assert!(clip.sample_rate == istft.sample_rate, "Sample Rate: {} {}", clip.sample_rate, istft.sample_rate);

        // there is no need to check max amplitude for exact correctness because the channels have been merged with the ifft, which means
        // amplitudes are averaged out
        assert!(istft.max_amplitude <= clip.max_amplitude, "Max Amp: {} {}", clip.max_amplitude, istft.max_amplitude);
    }

    #[test]
    fn test_medfilt() {
        let window_size = 101;
        let hop_size = window_size / 4;
        let source = Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");

        let stft = clip.stft(window_size, hop_size, rectangle_window);
        let norms = get_norms(&stft);

        let filt_v = medfilt_v(&norms, window_size);
        let filt_h = medfilt_h(&norms, window_size);

        assert_eq!(filt_v.len(), filt_h.len());
        assert_eq!(filt_v[0].len(), filt_h[0].len());
        assert_eq!(filt_v[0][0].len(), filt_h[0][0].len());
    }
    
    #[test]
    fn test_binary_mask() {
        let window_size = 101;
        let hop_size = window_size / 4;
        let source = Path::new("test_audio_clips/a6.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");

        let stft = clip.stft(window_size, hop_size, rectangle_window);
        let norms = get_norms(&stft);

        let filt_v = medfilt_v(&norms, window_size);
        let filt_h = medfilt_h(&norms, window_size);

        let (binary_h, binary_v) = binary_mask(&filt_h, &filt_v);

        assert_eq!(binary_v.len(), binary_h.len());
        assert_eq!(binary_v[0].len(), binary_h[0].len());
        assert_eq!(binary_v[0][0].len(), binary_h[0][0].len());
    }

    #[test]
    #[ignore]
    fn test_separate_harmonic_percussion() {
        let source = Path::new("test_audio_clips/comboTones.mp3");
        let clip = AudioClip::new(&source).expect("failed to create audio clip");

        let hop = 1025;
        let window_size = 1025;
        let (harmonic, percussion) = separate_harmonic_percussion(&clip, window_size, hop);
        let harmonic_path = Path::new("test_harmonic.wav");
        let percussion_path = Path::new("test_percussion.wav");
        harmonic.write(Some(harmonic_path)).unwrap();
        percussion.write(Some(percussion_path)).unwrap();
    }
}