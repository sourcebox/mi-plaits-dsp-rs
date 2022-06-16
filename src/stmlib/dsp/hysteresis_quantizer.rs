//! Hysteresis quantizer.
//!
//! Quantize a float in [0, 1] to an integer in [0, num_steps[. Apply hysteresis
//! to prevent jumps near the decision boundary.

// Based on MIT-licensed code (c) 2015 by Olivier Gillet (ol.gillet@gmail.com)

#[derive(Debug, Default)]
pub struct HysteresisQuantizer {
    quantized_value: i32,
}

impl HysteresisQuantizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.quantized_value = 0;
    }

    #[inline]
    pub fn process_with_default(&mut self, value: f32, num_steps: usize) -> i32 {
        self.process_with_hysteresis(value, num_steps, 0.25)
    }

    #[inline]
    pub fn process_with_hysteresis(
        &mut self,
        value: f32,
        num_steps: usize,
        hysteresis: f32,
    ) -> i32 {
        self.process(0, value, num_steps, hysteresis)
    }

    #[inline]
    pub fn process(&mut self, base: i32, value: f32, num_steps: usize, hysteresis: f32) -> i32 {
        let mut value = value * (num_steps - 1) as f32;
        value += base as f32;
        let hysteresis_feedback = if value > (self.quantized_value as f32) {
            -hysteresis
        } else {
            hysteresis
        };
        let q = ((value + hysteresis_feedback + 0.5) as i32).clamp(0, (num_steps - 1) as i32);
        self.quantized_value = q;

        q
    }
}
