//! Harmonic oscillator based on Chebyshev polynomials.
//!
//! Works well for a small number of harmonics. For the higher order harmonics,
//! we need to reinitialize the recurrence by computing two high harmonics.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::oscillator::sine_oscillator::{sine, sine_no_wrap};
use crate::utils::parameter_interpolator::{ParameterInterpolator, SimpleParameterInterpolator};

#[derive(Debug)]
pub struct HarmonicOscillator<const NUM_HARMONICS: usize> {
    // Oscillator state.
    phase: f32,

    // For interpolation of parameters.
    frequency: f32,
    amplitude: [f32; NUM_HARMONICS],
}

impl<const NUM_HARMONICS: usize> Default for HarmonicOscillator<NUM_HARMONICS> {
    fn default() -> Self {
        Self {
            phase: 0.0,
            frequency: 0.0,
            amplitude: [0.0; NUM_HARMONICS],
        }
    }
}

impl<const NUM_HARMONICS: usize> HarmonicOscillator<NUM_HARMONICS> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.frequency = 0.0;
        for elem in self.amplitude.iter_mut() {
            *elem = 0.0;
        }
    }

    #[inline]
    pub fn render(
        &mut self,
        mut frequency: f32,
        amplitudes: &[f32],
        out: &mut [f32],
        first_harmonic_index: usize,
    ) {
        if frequency >= 0.5 {
            frequency = 0.5;
        }

        let mut am: [SimpleParameterInterpolator; NUM_HARMONICS] =
            [SimpleParameterInterpolator::default(); NUM_HARMONICS];
        let mut fm = ParameterInterpolator::new(&mut self.frequency, frequency, out.len());

        let a = &mut self.amplitude;

        for i in 0..NUM_HARMONICS {
            let mut f = frequency * (first_harmonic_index + i) as f32;
            if f >= 0.5 {
                f = 0.5;
            }
            am[i].init(a[i], amplitudes[i] * (1.0 - f * 2.0), out.len());
        }

        for out_sample in out.iter_mut() {
            self.phase += fm.next();
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            let two_x = 2.0 * sine_no_wrap(self.phase);
            let mut previous;
            let mut current;
            if first_harmonic_index == 1 {
                previous = 1.0;
                current = two_x * 0.5;
            } else {
                let k = first_harmonic_index as f32;
                previous = sine(self.phase * (k - 1.0) + 0.25);
                current = sine(self.phase * k);
            }

            let mut sum = 0.0;
            for (i, am) in am.iter().enumerate().take(NUM_HARMONICS) {
                {
                    sum += am.update(&mut self.amplitude[i]) * current;
                    let temp = current;
                    current = two_x * current - previous;
                    previous = temp;
                }
                if first_harmonic_index == 1 {
                    *out_sample = sum;
                } else {
                    *out_sample += sum;
                }
            }
        }
    }
}
