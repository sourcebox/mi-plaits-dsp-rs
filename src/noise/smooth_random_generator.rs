//! Smooth random generator for the internal modulations.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::utils::random;

#[derive(Debug, Default, Clone)]
pub struct SmoothRandomGenerator {
    phase: f32,
    from: f32,
    interval: f32,
}

impl SmoothRandomGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.from = 0.0;
        self.interval = 0.0;
    }

    #[inline]
    pub fn render(&mut self, frequency: f32) -> f32 {
        self.phase += frequency;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.from += self.interval;
            self.interval = random::get_float() * 2.0 - 1.0 - self.from;
        }

        let t = self.phase * self.phase * (3.0 - 2.0 * self.phase);

        self.from + self.interval * t
    }
}
