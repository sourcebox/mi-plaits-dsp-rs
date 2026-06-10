//! Delay line.

// Based on MIT-licensed code (c) 2014 by Olivier Gillet (ol.gillet@gmail.com)

use alloc::vec::Vec;

use num_traits::{FromPrimitive, Num, Signed, ToPrimitive};

/// Delay line with a base capacity of `BASE_MAX_DELAY` samples at the
/// reference sample rate (48kHz). Call [`DelayLine::set_scale`] with
/// `sample_rate_hz / 48000.0` to keep the maximum delay time constant in
/// seconds at other sample rates.
#[derive(Debug, Clone)]
pub struct DelayLine<T, const BASE_MAX_DELAY: usize> {
    write_ptr: usize,
    delay: usize,
    line: Vec<T>,
}

impl<T, const BASE_MAX_DELAY: usize> Default for DelayLine<T, BASE_MAX_DELAY>
where
    T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const BASE_MAX_DELAY: usize> DelayLine<T, BASE_MAX_DELAY>
where
    T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
{
    pub fn new() -> Self {
        let mut line = Vec::new();
        line.resize(BASE_MAX_DELAY, T::zero());
        Self {
            write_ptr: 0,
            delay: 1,
            line,
        }
    }

    pub fn init(&mut self) {
        self.reset();
    }

    /// Resize the line so that its maximum delay time in seconds stays the
    /// same at a different sample rate. `scale` is `sample_rate_hz / 48000.0`.
    /// The line content is cleared.
    pub fn set_scale(&mut self, scale: f32) {
        let length = ((BASE_MAX_DELAY as f32 * scale) + 0.5) as usize;
        let length = length.max(1);
        self.line.clear();
        self.line.resize(length, T::zero());
        self.delay = 1;
        self.write_ptr = 0;
    }

    pub fn reset(&mut self) {
        for elem in self.line.iter_mut() {
            *elem = T::zero();
        }
        self.delay = 1;
        self.write_ptr = 0;
    }

    pub fn max_delay(&self) -> usize {
        self.line.len()
    }

    #[inline]
    pub fn set_delay(&mut self, delay: usize) {
        self.delay = delay;
    }

    #[inline]
    pub fn write(&mut self, sample: T) {
        let max_delay = self.line.len();
        self.line[self.write_ptr] = sample;
        self.write_ptr = (self.write_ptr + max_delay - 1) % max_delay;
    }

    #[inline]
    pub fn allpass(&mut self, sample: T, delay: usize, coefficient: T) -> T {
        let read = self.line[(self.write_ptr + delay) % self.line.len()];
        let write = sample + coefficient * read;
        self.write(write);

        -write * coefficient + read
    }

    #[inline]
    pub fn write_read(&mut self, sample: T, delay: f32) -> T {
        self.write(sample);
        self.read_with_delay_frac(delay)
    }

    #[inline]
    pub fn read(&self) -> T {
        self.line[(self.write_ptr + self.delay) % self.line.len()]
    }

    #[inline]
    pub fn read_with_delay(&self, delay: usize) -> T {
        self.line[(self.write_ptr + delay) % self.line.len()]
    }

    #[inline]
    pub fn read_with_delay_frac(&self, delay: f32) -> T {
        let max_delay = self.line.len();
        let delay_integral = delay as usize;
        let delay_fractional = delay - (delay_integral as f32);
        let a = self.line[(self.write_ptr + delay_integral) % max_delay];
        let b = self.line[(self.write_ptr + delay_integral + 1) % max_delay];

        let frac = (b - a).to_f32().unwrap_or_default() * delay_fractional;

        T::from_f32(a.to_f32().unwrap_or_default() + frac).unwrap_or_default()
    }

    #[inline]
    pub fn read_hermite(&self, delay: f32) -> T {
        let max_delay = self.line.len();
        let delay_integral = delay as usize;
        let delay_fractional = delay - (delay_integral as f32);
        let t = self.write_ptr + delay_integral + max_delay;
        let xm1 = self.line[(t - 1) % max_delay];
        let x0 = self.line[(t) % max_delay];
        let x1 = self.line[(t + 1) % max_delay];
        let x2 = self.line[(t + 2) % max_delay];
        let c = T::from_f32((x1 - xm1).to_f32().unwrap_or_default() * 0.5).unwrap_or_default();
        let v = x0 - x1;
        let w = c + v;
        let a = T::from_f32(
            (w + v).to_f32().unwrap_or_default() + ((x2 - x0).to_f32().unwrap_or_default() * 0.5),
        )
        .unwrap_or_default();
        let b_neg = w + a;
        let f = delay_fractional;

        T::from_f32(
            (((a.to_f32().unwrap_or_default() * f) - b_neg.to_f32().unwrap_or_default()) * f
                + c.to_f32().unwrap_or_default())
                * f
                + x0.to_f32().unwrap_or_default(),
        )
        .unwrap_or_default()
    }
}
