//! Noise processed by a sample and hold running at a target frequency.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::polyblep::{next_blep_sample, this_blep_sample};
use crate::utils::random;

#[derive(Debug, Default, Clone)]
pub struct ClockedNoise {
    // Oscillator state.
    phase: f32,
    sample: f32,
    next_sample: f32,

    // For interpolation of parameters.
    frequency: f32,
}

impl ClockedNoise {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.sample = 0.0;
        self.next_sample = 0.0;
        self.frequency = 0.001;
    }

    #[inline]
    pub fn render(&mut self, sync: bool, mut frequency: f32, out: &mut [f32]) {
        frequency = frequency.clamp(0.0, 1.0);

        let mut fm = ParameterInterpolator::new(&mut self.frequency, frequency, out.len());

        let mut next_sample = self.next_sample;
        let mut sample = self.sample;

        if sync {
            self.phase = 1.0;
        }

        for out_sample in out.iter_mut() {
            let mut this_sample = next_sample;
            next_sample = 0.0;

            let frequency = fm.next();
            let raw_sample = random::get_float() * 2.0 - 1.0;
            let raw_amount = 4.0 * (frequency - 0.25);
            let raw_amount = raw_amount.clamp(0.0, 1.0);

            self.phase += frequency;

            if self.phase >= 1.0 {
                self.phase -= 1.0;
                let t = self.phase / frequency;
                let new_sample = raw_sample;
                let discontinuity = new_sample - sample;
                this_sample += discontinuity * this_blep_sample(t);
                next_sample += discontinuity * next_blep_sample(t);
                sample = new_sample;
            }
            next_sample += sample;
            *out_sample = this_sample + raw_amount * (raw_sample - this_sample);
        }

        self.next_sample = next_sample;
        self.sample = sample;
    }
}
