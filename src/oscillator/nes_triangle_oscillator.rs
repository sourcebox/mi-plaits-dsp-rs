//! Triangle waveform approximated by discrete steps.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::polyblep::{
    next_blep_sample, next_integrated_blep_sample, this_blep_sample, this_integrated_blep_sample,
};

#[derive(Debug, Default)]
pub struct NesTriangleOscillator {
    phase: f32,
    next_sample: f32,
    step: i32,
    ascending: bool,

    frequency: f32,
}

impl NesTriangleOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.step = 0;
        self.ascending = true;
        self.next_sample = 0.0;
        self.frequency = 0.001;
    }

    #[inline]
    pub fn render(&mut self, mut frequency: f32, out: &mut [f32], num_bits: u32) {
        // Compute all constants needed to scale the waveform and its
        // discontinuities.
        let num_steps = 1 << num_bits;
        let half = num_steps / 2;
        let top = if num_steps != 2 { num_steps - 1 } else { 2 };
        let num_steps_f = num_steps as f32;
        let scale = if num_steps != 2 {
            4.0 / (top - 1) as f32
        } else {
            2.0
        };

        frequency = f32::min(frequency, 0.25);

        let mut fm = ParameterInterpolator::new(&mut self.frequency, frequency, out.len());

        let mut next_sample = self.next_sample;

        for out_sample in out.iter_mut() {
            let frequency = fm.next();
            self.phase += frequency;

            // Compute the point at which we transition between the "full resolution"
            // NES triangle, and a naive band-limited triangle.
            let fade_to_tri = ((frequency - 0.5 / num_steps_f) * 2.0 * num_steps_f).clamp(0.0, 1.0);

            let nes_gain = 1.0 - fade_to_tri;
            let tri_gain = fade_to_tri * 2.0 / scale;

            let mut this_sample = next_sample;
            next_sample = 0.0;

            // Handle the discontinuity at the top of the naive triangle.
            if self.ascending && self.phase >= 0.5 {
                let discontinuity = 4.0 * frequency * tri_gain;
                if discontinuity != 0.0 {
                    let t = (self.phase - 0.5) / frequency;
                    this_sample -= this_integrated_blep_sample(t) * discontinuity;
                    next_sample -= next_integrated_blep_sample(t) * discontinuity;
                }
                self.ascending = false;
            }

            let mut next_step = (self.phase * num_steps_f) as i32;

            if next_step != self.step {
                let mut wrap = false;

                if next_step >= num_steps {
                    self.phase -= 1.0;
                    next_step -= num_steps;
                    wrap = true;
                }

                let mut discontinuity = if next_step < half { 1.0 } else { -1.0 };

                if num_steps == 2 {
                    discontinuity = -discontinuity;
                } else if next_step == 0 || next_step == half {
                    discontinuity = 0.0;
                }

                // Handle the discontinuity at each step of the NES triangle.
                discontinuity *= nes_gain;

                if discontinuity != 0.0 {
                    let frac = self.phase * num_steps_f - next_step as f32;
                    let t = frac / (frequency * num_steps_f);
                    this_sample += this_blep_sample(t) * discontinuity;
                    next_sample += next_blep_sample(t) * discontinuity;
                }

                // Handle the discontinuity at the bottom of the naive triangle.
                if wrap {
                    let discontinuity = 4.0 * frequency * tri_gain;
                    if discontinuity != 0.0 {
                        let t = self.phase / frequency;
                        this_sample += this_integrated_blep_sample(t) * discontinuity;
                        next_sample += next_integrated_blep_sample(t) * discontinuity;
                    }
                    self.ascending = true;
                }
            }

            self.step = next_step;

            // Contribution from NES triangle.
            next_sample += nes_gain
                * (if self.step < half {
                    self.step
                } else {
                    top - self.step
                }) as f32;

            // Contribution from naive triangle.
            next_sample += tri_gain
                * (if self.phase < 0.5 {
                    2.0 * self.phase
                } else {
                    2.0 - 2.0 * self.phase
                });

            *out_sample = this_sample * scale - 1.0;
        }

        self.next_sample = next_sample;
    }
}
