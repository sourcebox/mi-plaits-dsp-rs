//! Sinewave multiplied by and sync'ed to a carrier.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::oscillator::oscillator::MAX_FREQUENCY;
use crate::oscillator::sine_oscillator::sine;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::polyblep::{next_blep_sample, this_blep_sample};

#[derive(Debug, Default)]
pub struct ZOscillator {
    // Oscillator state.
    carrier_phase: f32,
    discontinuity_phase: f32,
    formant_phase: f32,
    next_sample: f32,

    // For interpolation of parameters.
    carrier_frequency: f32,
    formant_frequency: f32,
    carrier_shape: f32,
    mode: f32,
}

impl ZOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.carrier_phase = 0.0;
        self.discontinuity_phase = 0.0;
        self.formant_phase = 0.0;
        self.next_sample = 0.0;

        self.carrier_frequency = 0.0;
        self.formant_frequency = 0.0;
        self.carrier_shape = 0.0;
        self.mode = 0.0;
    }

    #[inline]
    pub fn render(
        &mut self,
        mut carrier_frequency: f32,
        mut formant_frequency: f32,
        carrier_shape: f32,
        mode: f32,
        out: &mut [f32],
    ) {
        if carrier_frequency >= MAX_FREQUENCY * 0.5 {
            carrier_frequency = MAX_FREQUENCY * 0.5;
        }
        if formant_frequency >= MAX_FREQUENCY {
            formant_frequency = MAX_FREQUENCY;
        }

        let mut carrier_frequency_modulation =
            ParameterInterpolator::new(&mut self.carrier_frequency, carrier_frequency, out.len());
        let mut formant_frequency_modulation =
            ParameterInterpolator::new(&mut self.formant_frequency, formant_frequency, out.len());
        let mut carrier_shape_modulation =
            ParameterInterpolator::new(&mut self.carrier_shape, carrier_shape, out.len());
        let mut mode_modulation = ParameterInterpolator::new(&mut self.mode, mode, out.len());

        let mut next_sample = self.next_sample;

        for out_sample in out.iter_mut() {
            let reset_time;

            let mut this_sample = next_sample;
            next_sample = 0.0;

            let f0 = carrier_frequency_modulation.next();
            let f1 = formant_frequency_modulation.next();

            self.discontinuity_phase += 2.0 * f0;
            self.carrier_phase += f0;
            let reset = self.discontinuity_phase >= 1.0;

            if reset {
                self.discontinuity_phase -= 1.0;
                reset_time = self.discontinuity_phase / (2.0 * f0);

                let carrier_phase_before = if self.carrier_phase >= 1.0 { 1.0 } else { 0.5 };
                let carrier_phase_after = if self.carrier_phase >= 1.0 { 0.0 } else { 0.5 };
                let before = z(
                    carrier_phase_before,
                    1.0,
                    self.formant_phase + (1.0 - reset_time) * f1,
                    carrier_shape_modulation.subsample(1.0 - reset_time),
                    mode_modulation.subsample(1.0 - reset_time),
                );

                let after = z(
                    carrier_phase_after,
                    0.0,
                    0.0,
                    carrier_shape_modulation.subsample(1.0),
                    mode_modulation.subsample(1.0),
                );

                let discontinuity = after - before;
                this_sample += discontinuity * this_blep_sample(reset_time);
                next_sample += discontinuity * next_blep_sample(reset_time);
                self.formant_phase = reset_time * f1;

                if self.carrier_phase > 1.0 {
                    self.carrier_phase = self.discontinuity_phase * 0.5;
                }
            } else {
                self.formant_phase += f1;
                if self.formant_phase >= 1.0 {
                    self.formant_phase -= 1.0;
                }
            }

            if self.carrier_phase >= 1.0 {
                self.carrier_phase -= 1.0;
            }

            next_sample += z(
                self.carrier_phase,
                self.discontinuity_phase,
                self.formant_phase,
                carrier_shape_modulation.next(),
                mode_modulation.next(),
            );
            *out_sample = this_sample;
        }

        self.next_sample = next_sample;
    }
}

#[inline]
fn z(c: f32, d: f32, f: f32, mut shape: f32, mode: f32) -> f32 {
    let mut ramp_down = 0.5 * (1.0 + sine(0.5 * d + 0.25));

    let offset;
    let phase_shift;

    if mode < 0.333 {
        offset = 1.0;
        phase_shift = 0.25 + mode * 1.50;
    } else if mode < 0.666 {
        phase_shift = 0.7495 - (mode - 0.33) * 0.75;
        offset = -sine(phase_shift);
    } else {
        phase_shift = 0.7495 - (mode - 0.33) * 0.75;
        offset = 0.001;
    }

    let discontinuity = sine(f + phase_shift);
    let contour = if shape < 0.5 {
        shape *= 2.0;
        if c >= 0.5 {
            ramp_down *= shape;
        }
        1.0 + (sine(c + 0.25) - 1.0) * shape
    } else {
        sine(c + shape * 0.5)
    };

    (ramp_down * (offset + discontinuity) - offset) * contour
}
