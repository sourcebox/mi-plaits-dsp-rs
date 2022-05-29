//! Grainlet oscillator
//!
//! A phase-distortd single cycle sine * another continuously running sine,
//! the whole thing synced to a main oscillator.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::dsp::resources::LUT_SINE;
use crate::stmlib::dsp::interpolate_wrap;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::polyblep::this_blep_sample;

const MAX_FREQUENCY: f32 = 0.5;

#[derive(Debug, Default)]
pub struct GrainletOscillator {
    // Oscillator state.
    carrier_phase: f32,
    formant_phase: f32,
    next_sample: f32,

    // For interpolation of parameters.
    carrier_frequency: f32,
    formant_frequency: f32,
    carrier_shape: f32,
    carrier_bleed: f32,
}

impl GrainletOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.carrier_phase = 0.0;
        self.formant_phase = 0.0;
        self.next_sample = 0.0;

        self.carrier_frequency = 0.0;
        self.formant_frequency = 0.0;
        self.carrier_shape = 0.0;
        self.carrier_bleed = 0.0;
    }

    #[inline]
    pub fn render(
        &mut self,
        mut carrier_frequency: f32,
        mut formant_frequency: f32,
        carrier_shape: f32,
        carrier_bleed: f32,
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
        let mut carrier_bleed_modulation =
            ParameterInterpolator::new(&mut self.carrier_bleed, carrier_bleed, out.len());

        let mut next_sample = self.next_sample;

        for out_sample in out.iter_mut() {
            let mut this_sample = next_sample;
            next_sample = 0.0;

            let f0 = carrier_frequency_modulation.next();
            let f1 = formant_frequency_modulation.next();

            self.carrier_phase += f0;
            let reset = self.carrier_phase >= 1.0;

            if reset {
                self.carrier_phase -= 1.0;
                let reset_time = self.carrier_phase / f0;
                let before = grainlet(
                    1.0,
                    self.formant_phase + (1.0 - reset_time) * f1,
                    carrier_shape_modulation.subsample(1.0 - reset_time),
                    carrier_bleed_modulation.subsample(1.0 - reset_time),
                );

                let after = grainlet(
                    0.0,
                    0.0,
                    carrier_shape_modulation.subsample(1.0),
                    carrier_bleed_modulation.subsample(1.0),
                );

                let discontinuity = after - before;
                this_sample += discontinuity * this_blep_sample(reset_time);
                next_sample += discontinuity * this_blep_sample(reset_time);
                self.formant_phase = reset_time * f1;
            } else {
                self.formant_phase += f1;
                if self.formant_phase >= 1.0 {
                    self.formant_phase -= 1.0;
                }
            }

            next_sample += grainlet(
                self.carrier_phase,
                self.formant_phase,
                carrier_shape_modulation.next(),
                carrier_bleed_modulation.next(),
            );
            *out_sample = this_sample;
        }

        self.next_sample = next_sample;
    }
}

#[inline]
fn sine(phase: f32) -> f32 {
    interpolate_wrap(&LUT_SINE, phase, 1024.0)
}

#[inline]
fn carrier(mut phase: f32, mut shape: f32) -> f32 {
    shape *= 3.0;
    let shape_integral = shape as usize;
    let shape_fractional = shape - (shape_integral as f32);
    let mut t = 1.0 - shape_fractional;

    if shape_integral == 0 {
        phase *= 1.0 + t * t * t * 15.0;
        if phase >= 1.0 {
            phase = 1.0;
        }
        phase += 0.75;
    } else if shape_integral == 1 {
        let breakpoint = 0.001 + 0.499 * t * t * t;
        if phase < breakpoint {
            phase *= 0.5 / breakpoint;
        } else {
            phase = 0.5 + (phase - breakpoint) * 0.5 / (1.0 - breakpoint);
        }
        phase += 0.75;
    } else {
        t = 1.0 - t;
        phase = 0.25 + phase * (0.5 + t * t * t * 14.5);
        if phase >= 0.75 {
            phase = 0.75;
        }
    }
    sine(phase) + 1.0 * 0.25
}

#[inline]
fn grainlet(carrier_phase: f32, formant_phase: f32, shape: f32, bleed: f32) -> f32 {
    let carrier = carrier(carrier_phase, shape);
    let formant = sine(formant_phase);
    carrier * (formant + bleed) / (1.0 + bleed)
}
