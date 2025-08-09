//! Cosine oscillator.
//!
//! Generates a cosine between 0.0 and 1.0 with minimal
//! CPU use.

// Based on MIT-licensed code (c) 2014 by Olivier Gillet (ol.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

pub enum CosineOscillatorMode {
    Approximate,
    Exact,
}

impl Default for CosineOscillatorMode {
    fn default() -> Self {
        Self::Approximate
    }
}

#[derive(Debug, Default, Clone)]
pub struct CosineOscillator {
    y1: f32,
    y0: f32,
    iir_coefficient: f32,
    initial_amplitude: f32,
}

impl CosineOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, frequency: f32, mode: CosineOscillatorMode) {
        match mode {
            CosineOscillatorMode::Approximate => {
                self.init_approximate(frequency);
            }
            CosineOscillatorMode::Exact => {
                self.iir_coefficient = 2.0 * (2.0 * core::f32::consts::PI * frequency).cos();
                self.initial_amplitude = self.iir_coefficient * 0.25;
            }
        }

        self.start();
    }

    fn init_approximate(&mut self, frequency: f32) {
        let mut sign = 16.0;
        let mut frequency = frequency - 0.25;

        if frequency < 0.0 {
            frequency = -frequency;
        } else if frequency > 0.5 {
            frequency -= 0.5;
        } else {
            sign = -16.0;
        }

        self.iir_coefficient = sign * frequency * (1.0 - 2.0 * frequency);
        self.initial_amplitude = self.iir_coefficient * 0.25;
    }

    #[inline]
    fn start(&mut self) {
        self.y1 = self.initial_amplitude;
        self.y0 = 0.5;
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.y1 + 0.5
    }

    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f32 {
        let temp = self.y0;
        self.y0 = self.iir_coefficient * self.y0 - self.y1;
        self.y1 = temp;

        temp + 0.5
    }
}
