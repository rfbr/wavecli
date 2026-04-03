/// Downsample a channel's samples to `width` columns.
/// Returns (min, max) amplitude pairs for each column.
pub fn compute_waveform(samples: &[f32], width: usize) -> Vec<(f32, f32)> {
    if width == 0 || samples.is_empty() {
        return Vec::new();
    }

    let samples_per_col = samples.len() as f64 / width as f64;
    let mut result = Vec::with_capacity(width);

    for col in 0..width {
        let start = (col as f64 * samples_per_col) as usize;
        let end = (((col + 1) as f64 * samples_per_col) as usize).min(samples.len());

        if start >= end {
            result.push((0.0, 0.0));
            continue;
        }

        let mut min = f32::MAX;
        let mut max = f32::MIN;

        for &s in &samples[start..end] {
            if s < min {
                min = s;
            }
            if s > max {
                max = s;
            }
        }

        result.push((min, max));
    }

    result
}
