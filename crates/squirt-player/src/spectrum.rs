//! A spectrum for the visualizer. The audio callback copies what it plays
//! into a small [`Tap`] (mono, before volume, so the bars don't shrink when
//! you turn it down); [`bands`] turns the latest window into band levels.

use std::f32::consts::PI;

/// Samples per analysis window (about 43 ms at 48 kHz).
pub const WINDOW: usize = 2048;

/// The most recent [`WINDOW`] mono samples, as a ring.
pub struct Tap {
    buf: Vec<f32>,
    pos: usize,
}

impl Tap {
    pub fn new() -> Self {
        Self { buf: vec![0.0; WINDOW], pos: 0 }
    }

    /// Adds interleaved samples, mixed down to mono. Does not allocate (it
    /// runs in the audio callback).
    pub fn push(&mut self, interleaved: &[f32], channels: usize) {
        let ch = channels.max(1);
        for frame in interleaved.chunks_exact(ch) {
            self.buf[self.pos] = frame.iter().sum::<f32>() / ch as f32;
            self.pos = (self.pos + 1) % WINDOW;
        }
    }

    /// The window, oldest sample first.
    pub fn latest(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(WINDOW);
        v.extend_from_slice(&self.buf[self.pos..]);
        v.extend_from_slice(&self.buf[..self.pos]);
        v
    }

    pub fn clear(&mut self) {
        self.buf.fill(0.0);
    }
}

/// In-place radix-2 FFT; `re.len()` must be a power of two.
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len <<= 1;
    }
}

/// Levels (0 to 1) of `n` log-spaced bands from 40 Hz to 16 kHz, mapped
/// from -70..-12 dBFS. A full-scale sine reads about 1.
pub fn bands(samples: &[f32], rate: u32, n: usize) -> Vec<f32> {
    let len = samples.len();
    if n == 0 || len < 2 || !len.is_power_of_two() {
        return vec![0.0; n];
    }
    // Hann window; its coherent gain is 0.5.
    let mut re: Vec<f32> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| s * (0.5 - 0.5 * (2.0 * PI * i as f32 / (len - 1) as f32).cos()))
        .collect();
    let mut im = vec![0.0; len];
    fft(&mut re, &mut im);
    let scale = 2.0 / (len as f32 * 0.5);
    let mag: Vec<f32> = (0..len / 2).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt() * scale).collect();

    let bin_hz = rate as f32 / len as f32;
    let (lo, hi) = (40.0f32, 16_000f32.min(rate as f32 / 2.0));
    let ratio = (hi / lo).powf(1.0 / n as f32);
    (0..n)
        .map(|i| {
            let f0 = lo * ratio.powi(i as i32);
            let f1 = f0 * ratio;
            let (b0, b1) = ((f0 / bin_hz).floor() as usize, (f1 / bin_hz).ceil() as usize);
            // Low bands can be narrower than one bin: use the nearest bin.
            let peak = mag[b0.min(mag.len() - 1)..b1.clamp(b0 + 1, mag.len())].iter().copied().fold(0.0f32, f32::max);
            let db = 20.0 * peak.max(1e-9).log10();
            ((db + 70.0) / 58.0).clamp(0.0, 1.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sine_lights_its_band() {
        let rate = 48_000;
        let sine: Vec<f32> = (0..WINDOW).map(|i| (2.0 * PI * 1000.0 * i as f32 / rate as f32).sin() * 0.5).collect();
        let b = bands(&sine, rate, 32);
        let (peak, _) = b.iter().enumerate().fold((0, 0.0), |m, (i, &v)| if v > m.1 { (i, v) } else { m });
        // 1 kHz sits about 54% of the way up a 40 Hz-16 kHz log scale.
        let expected = ((1000.0f32 / 40.0).ln() / (16_000.0f32 / 40.0).ln() * 32.0) as usize;
        assert!(peak.abs_diff(expected) <= 1, "peak band {peak}, expected {expected}: {b:?}");
        assert!(b[peak] > 0.8);
        assert!(b[2] < 0.3 && b[30] < 0.3);
        assert!(bands(&vec![0.0; WINDOW], rate, 32).iter().all(|&v| v == 0.0));
    }

    #[test]
    fn tap_keeps_the_latest_window_in_order() {
        let mut t = Tap::new();
        let stereo: Vec<f32> = (0..WINDOW + 10).flat_map(|i| [i as f32, i as f32]).collect();
        t.push(&stereo, 2);
        let v = t.latest();
        assert_eq!(v[0], 10.0);
        assert_eq!(v[WINDOW - 1], (WINDOW + 9) as f32);
    }
}
