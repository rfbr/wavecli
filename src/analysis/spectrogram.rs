use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::f32::consts::PI;

pub struct SpectrogramData {
    /// 2D grid: magnitudes[time_bin][freq_bin] in dB
    pub magnitudes: Vec<Vec<f32>>,
    pub num_time_bins: usize,
    pub num_freq_bins: usize,
    pub min_db: f32,
    pub max_db: f32,
    pub max_freq: f32,
}

/// Compute spectrogram from mono samples.
/// `window_size`: FFT window size (e.g. 1024)
/// `hop_size`: hop between windows (typically window_size / 2)
pub fn compute_spectrogram(
    samples: &[f32],
    sample_rate: u32,
    window_size: usize,
    hop_size: usize,
) -> SpectrogramData {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    let num_freq_bins = window_size / 2;
    let num_time_bins = if samples.len() >= window_size {
        (samples.len() - window_size) / hop_size + 1
    } else {
        0
    };

    // Precompute Hann window
    let hann: Vec<f32> = (0..window_size)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (window_size - 1) as f32).cos()))
        .collect();

    let mut magnitudes = Vec::with_capacity(num_time_bins);
    let mut global_min = f32::MAX;
    let mut global_max = f32::MIN;

    let mut buffer = vec![Complex::new(0.0f32, 0.0f32); window_size];

    for t in 0..num_time_bins {
        let offset = t * hop_size;

        // Apply window and fill FFT buffer
        for i in 0..window_size {
            buffer[i] = Complex::new(samples[offset + i] * hann[i], 0.0);
        }

        fft.process(&mut buffer);

        // Compute magnitude in dB for positive frequencies
        let freq_bins: Vec<f32> = buffer[..num_freq_bins]
            .iter()
            .map(|c| {
                let mag = c.norm() / window_size as f32;
                let db = 20.0 * mag.max(1e-10).log10();
                db
            })
            .collect();

        for &db in &freq_bins {
            if db < global_min {
                global_min = db;
            }
            if db > global_max {
                global_max = db;
            }
        }

        magnitudes.push(freq_bins);
    }

    let max_freq = sample_rate as f32 / 2.0;

    SpectrogramData {
        magnitudes,
        num_time_bins,
        num_freq_bins,
        min_db: global_min,
        max_db: global_max,
        max_freq,
    }
}
