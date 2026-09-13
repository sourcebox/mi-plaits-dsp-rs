//! Ensemble FX.

// Based on MIT-licensed code (c) 2014 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{DataFormat32Bit, FxContext, FxEngine};
use crate::oscillator::sine_oscillator::sine_raw;
use crate::utils::delay_line::DelayLine;

#[derive(Debug, Default, Clone)]
pub struct Ensemble {
    line_l: DelayLine<f32, 511>,
    line_r: DelayLine<f32, 511>,

    engine: FxEngine<1024, DataFormat32Bit>,

    amount: f32,
    depth: f32,

    phase_1: u32,
    phase_2: u32,
}

impl Ensemble {
    pub fn new() -> Self {
        Self {
            line_l: DelayLine::new(),
            line_r: DelayLine::new(),

            engine: FxEngine::new(),

            amount: 0.0,
            depth: 0.0,

            phase_1: 0,
            phase_2: 0,
        }
    }

    pub fn init(&mut self) {
        self.phase_1 = 0;
        self.phase_2 = 0;
    }

    pub fn reset(&mut self) {
        self.clear();
    }

    pub fn clear(&mut self) {
        self.line_l.reset();
        self.line_r.reset();
        self.engine.clear();
    }

    #[inline]
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let mut c = FxContext::new();

        for (left_sample, right_sample) in left.iter_mut().zip(right.iter_mut()) {
            self.engine.start(&mut c);
            let dry_amount = 1.0 - self.amount * 0.5;

            // Update LFO.
            let one_third = 1417339207;
            let two_third = 2834678415;

            self.phase_1 = self.phase_1.wrapping_add(67289); // 0.75 Hz
            self.phase_2 = self.phase_2.wrapping_add(589980); // 6.57 Hz
            let slow_0 = sine_raw(self.phase_1);
            let slow_120 = sine_raw(self.phase_1.wrapping_add(one_third));
            let slow_240 = sine_raw(self.phase_1.wrapping_add(two_third));
            let fast_0 = sine_raw(self.phase_2);
            let fast_120 = sine_raw(self.phase_2.wrapping_add(one_third));
            let fast_240 = sine_raw(self.phase_2.wrapping_add(two_third));

            // Max deviation: 176
            let a = self.depth * 160.0;
            let b = self.depth * 16.0;

            let mod_1 = slow_0 * a + fast_0 * b;
            let mod_2 = slow_120 * a + fast_120 * b;
            let mod_3 = slow_240 * a + fast_240 * b;

            let mut wet = 0.0;

            // Sum L & R channel to send to chorus line.
            c.read_with_scale(*left_sample, 1.0);
            c.write_line(&mut self.line_l, 0.0);
            c.read_with_scale(*right_sample, 1.0);
            c.write_line(&mut self.line_r, 0.0);

            c.interpolate(&mut self.line_l, mod_1 + 192.0, 0.0, 0.33);
            c.interpolate(&mut self.line_l, mod_2 + 192.0, 0.0, 0.33);
            c.interpolate(&mut self.line_r, mod_3 + 192.0, 0.0, 0.33);
            c.write(&mut wet);
            *left_sample = wet * self.amount + *left_sample * dry_amount;

            c.interpolate(&mut self.line_r, mod_1 + 192.0, 0.0, 0.33);
            c.interpolate(&mut self.line_r, mod_2 + 192.0, 0.0, 0.33);
            c.interpolate(&mut self.line_l, mod_3 + 192.0, 0.0, 0.33);
            c.write(&mut wet);
            *right_sample = wet * self.amount + *right_sample * dry_amount;
        }
    }

    #[inline]
    pub fn set_amount(&mut self, amount: f32) {
        self.amount = amount;
    }

    #[inline]
    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth;
    }
}

#[cfg(test)]
mod tests {
    use super::Ensemble;

    const BLOCK_SIZE: usize = 24;
    const RESET_PROBE_BLOCKS: usize = 400;

    #[test]
    fn reset_clears_delay_lines() {
        let mut fx = Ensemble::new();
        fx.init();
        fx.set_amount(1.0);
        fx.set_depth(0.5);

        let mut left = [1.0; BLOCK_SIZE];
        let mut right = [1.0; BLOCK_SIZE];
        fx.process(&mut left, &mut right);
        fx.reset();

        // A single block is not long enough for stale samples to emerge from the
        // chorus lines, so observe several complete delay lengths.
        for block in 0..RESET_PROBE_BLOCKS {
            left.fill(0.0);
            right.fill(0.0);
            fx.process(&mut left, &mut right);
            assert!(
                left.iter().all(|sample| *sample == 0.0)
                    && right.iter().all(|sample| *sample == 0.0),
                "ensemble produced a stale sample after reset in probe block {block}"
            );
        }
    }
}
