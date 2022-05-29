//! Limiter.

// Based on MIT-licensed code (c) 2015 by Olivier Gillet (ol.gillet@gmail.com)

use super::slope;

use num_traits::float::FloatCore;

#[derive(Debug, Default)]
pub struct Limiter {
    peak: f32,
}

impl Limiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.peak = 0.5;
    }

    #[inline]
    pub fn process(&mut self, pre_gain: f32, in_out: &mut [f32]) {
        for sample in in_out.iter_mut() {
            let s = *sample * pre_gain;
            slope(&mut self.peak, s.abs(), 0.05, 0.00002);
            let gain = if self.peak <= 1.0 {
                1.0
            } else {
                1.0 / self.peak
            };
            *sample = s * gain * 0.8;
        }
    }
}
