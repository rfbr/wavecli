use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rodio::source::Source;
use rodio::{OutputStream, OutputStreamHandle, Sink};

struct AudioSource {
    samples: Vec<f32>, // interleaved
    channels: u16,
    sample_rate: u32,
    position: Arc<AtomicU64>, // current sample index
    balance: Arc<AtomicU32>,  // f32 bits: -1.0 (left) to 1.0 (right)
}

impl AudioSource {
    fn balance_gain(&self, sample_idx: usize) -> f32 {
        if self.channels != 2 {
            return 1.0;
        }
        let balance = f32::from_bits(self.balance.load(Ordering::Relaxed));
        let ch = sample_idx % 2;
        if ch == 0 {
            // left channel
            (1.0 - balance).clamp(0.0, 1.0)
        } else {
            // right channel
            (1.0 + balance).clamp(0.0, 1.0)
        }
    }
}

impl Iterator for AudioSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let frame = self.position.load(Ordering::Relaxed);
        let sample_idx = frame as usize;

        if sample_idx >= self.samples.len() {
            return None;
        }

        let val = self.samples[sample_idx] * self.balance_gain(sample_idx);
        self.position.fetch_add(1, Ordering::Relaxed);
        Some(val)
    }
}

impl Source for AudioSource {
    fn current_frame_len(&self) -> Option<usize> {
        let pos = self.position.load(Ordering::Relaxed) as usize;
        let remaining = self.samples.len().saturating_sub(pos);
        if remaining == 0 {
            None
        } else {
            Some(remaining)
        }
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let total_frames = self.samples.len() / self.channels as usize;
        Some(Duration::from_secs_f64(
            total_frames as f64 / self.sample_rate as f64,
        ))
    }
}

/// Audio backend when a real device is available
struct AudioBackend {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sink: Sink,
}

pub struct Player {
    backend: Option<AudioBackend>,
    position: Arc<AtomicU64>,
    balance: Arc<AtomicU32>,
    channels: u16,
    sample_rate: u32,
    total_interleaved_samples: usize,
    volume: f32,
    // Fallback cursor tracking when no audio device
    playing: bool,
    last_tick: Instant,
}

impl Player {
    pub fn new(
        samples: &[Vec<f32>],
        sample_rate: u32,
        channels: usize,
    ) -> Self {
        let total_frames = samples.first().map(|c| c.len()).unwrap_or(0);

        // Interleave samples
        let mut interleaved = Vec::with_capacity(total_frames * channels);
        for frame in 0..total_frames {
            for ch in 0..channels {
                interleaved.push(samples[ch][frame]);
            }
        }

        let position = Arc::new(AtomicU64::new(0));
        let balance = Arc::new(AtomicU32::new(0.0_f32.to_bits()));

        // Suppress ALSA warnings that would corrupt the TUI
        let _stderr_guard = SuppressStderr::new();

        let backend = match OutputStream::try_default() {
            Ok((stream, stream_handle)) => {
                match Sink::try_new(&stream_handle) {
                    Ok(sink) => {
                        sink.pause();
                        let source = AudioSource {
                            samples: interleaved,
                            channels: channels as u16,
                            sample_rate,
                            position: position.clone(),
                            balance: balance.clone(),
                        };
                        sink.append(source);
                        Some(AudioBackend {
                            _stream: stream,
                            _stream_handle: stream_handle,
                            sink,
                        })
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        };

        drop(_stderr_guard);

        Player {
            backend,
            position,
            balance,
            channels: channels as u16,
            sample_rate,
            total_interleaved_samples: total_frames * channels,
            volume: 1.0,
            playing: false,
            last_tick: Instant::now(),
        }
    }

    pub fn has_audio_device(&self) -> bool {
        self.backend.is_some()
    }

    pub fn toggle_play(&mut self) {
        // Reset to start if at end
        let at_end =
            self.position.load(Ordering::Relaxed) as usize >= self.total_interleaved_samples;

        if let Some(ref backend) = self.backend {
            if backend.sink.is_paused() {
                if at_end {
                    self.position.store(0, Ordering::Relaxed);
                }
                backend.sink.play();
            } else {
                backend.sink.pause();
            }
        } else {
            self.playing = !self.playing;
            if self.playing {
                if at_end {
                    self.position.store(0, Ordering::Relaxed);
                }
                self.last_tick = Instant::now();
            }
        }
    }

    /// Call each frame to advance the fallback cursor
    pub fn tick(&mut self) {
        if self.backend.is_none() && self.playing {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_tick).as_secs_f64();
            self.last_tick = now;

            let samples_to_advance = (elapsed * self.sample_rate as f64 * self.channels as f64) as u64;
            let new_pos = self.position.load(Ordering::Relaxed) + samples_to_advance;
            if new_pos as usize >= self.total_interleaved_samples {
                self.position.store(self.total_interleaved_samples as u64, Ordering::Relaxed);
                self.playing = false;
            } else {
                self.position.store(new_pos, Ordering::Relaxed);
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        if let Some(ref backend) = self.backend {
            !backend.sink.is_paused() && !backend.sink.empty()
        } else {
            self.playing
        }
    }

    pub fn position_secs(&self) -> f64 {
        let sample_pos = self.position.load(Ordering::Relaxed) as usize;
        let frame_pos = sample_pos / self.channels as usize;
        frame_pos as f64 / self.sample_rate as f64
    }

    pub fn position_fraction(&self) -> f64 {
        let sample_pos = self.position.load(Ordering::Relaxed) as usize;
        if self.total_interleaved_samples == 0 {
            return 0.0;
        }
        sample_pos as f64 / self.total_interleaved_samples as f64
    }

    pub fn seek_to(&mut self, secs: f64) {
        let frame = (secs * self.sample_rate as f64) as usize;
        let sample_idx = frame * self.channels as usize;
        let clamped = sample_idx.min(self.total_interleaved_samples);
        self.position.store(clamped as u64, Ordering::Relaxed);
    }

    pub fn seek_relative(&mut self, delta_secs: f64) {
        let current = self.position_secs();
        let total = self.duration_secs();
        let new_pos = (current + delta_secs).clamp(0.0, total);
        self.seek_to(new_pos);
    }

    pub fn duration_secs(&self) -> f64 {
        let total_frames = self.total_interleaved_samples / self.channels as usize;
        total_frames as f64 / self.sample_rate as f64
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        if let Some(ref backend) = self.backend {
            backend.sink.set_volume(self.volume);
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn adjust_volume(&mut self, delta: f32) {
        self.set_volume(self.volume + delta);
    }

    pub fn balance(&self) -> f32 {
        f32::from_bits(self.balance.load(Ordering::Relaxed))
    }

    pub fn set_balance(&self, bal: f32) {
        let clamped = bal.clamp(-1.0, 1.0);
        self.balance.store(clamped.to_bits(), Ordering::Relaxed);
    }

    pub fn adjust_balance(&self, delta: f32) {
        self.set_balance(self.balance() + delta);
    }

    pub fn is_stereo(&self) -> bool {
        self.channels == 2
    }
}

/// Temporarily redirects stderr (fd 2) to /dev/null.
/// Restores on drop.
struct SuppressStderr {
    saved_fd: i32,
}

impl SuppressStderr {
    fn new() -> Self {
        unsafe {
            let saved_fd = libc::dup(2);
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const _, libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 2);
                libc::close(devnull);
            }
            SuppressStderr { saved_fd }
        }
    }
}

impl Drop for SuppressStderr {
    fn drop(&mut self) {
        if self.saved_fd >= 0 {
            unsafe {
                libc::dup2(self.saved_fd, 2);
                libc::close(self.saved_fd);
            }
        }
    }
}
