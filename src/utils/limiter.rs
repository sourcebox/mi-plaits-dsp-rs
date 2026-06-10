//! Limiter.

// Based on MIT-licensed code (c) 2015 by Olivier Gillet (ol.gillet@gmail.com)

use super::{slope, REFERENCE_SAMPLE_RATE};

#[allow(unused_imports)]
use num_traits::float::FloatCore;

#[derive(Debug, Clone)]
pub struct Limiter {
    peak: f32,

    // Sample rate dependent constants
    attack: f32,
    release: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        Self {
            peak: 0.5,
            attack: 0.05,
            release: 0.00002,
        }
    }
}

impl Limiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        // Keep attack/release times constant in seconds at any sample rate.
        let rate_ratio = REFERENCE_SAMPLE_RATE / sample_rate_hz;
        self.attack = 0.05 * rate_ratio;
        self.release = 0.00002 * rate_ratio;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.peak = 0.5;
    }

    #[inline]
    pub fn process(&mut self, pre_gain: f32, in_out: &mut [f32]) {
        for sample in in_out.iter_mut() {
            let s = *sample * pre_gain;
            slope(&mut self.peak, s.abs(), self.attack, self.release);
            let gain = if self.peak <= 1.0 {
                1.0
            } else {
                1.0 / self.peak
            };
            *sample = s * gain * 0.8;
        }
    }
}
