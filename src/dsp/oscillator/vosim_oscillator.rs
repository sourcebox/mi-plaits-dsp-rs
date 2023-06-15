//! Two sinewaves multiplied by and sync'ed to a carrier.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::dsp::oscillator::sine_oscillator::sine;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;

const MAX_FREQUENCY: f32 = 0.5;

#[derive(Debug, Default)]
pub struct VosimOscillator {
    // Oscillator state.
    carrier_phase: f32,
    formant_1_phase: f32,
    formant_2_phase: f32,

    // For interpolation of parameters.
    carrier_frequency: f32,
    formant_1_frequency: f32,
    formant_2_frequency: f32,
    carrier_shape: f32,
}

impl VosimOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.carrier_phase = 0.0;
        self.formant_1_phase = 0.0;
        self.formant_2_phase = 0.0;

        self.carrier_frequency = 0.0;
        self.formant_1_frequency = 0.0;
        self.formant_2_frequency = 0.0;
        self.carrier_shape = 0.0;
    }

    #[inline]
    pub fn render(
        &mut self,
        mut carrier_frequency: f32,
        mut formant_frequency_1: f32,
        mut formant_frequency_2: f32,
        carrier_shape: f32,
        out: &mut [f32],
    ) {
        if carrier_frequency >= MAX_FREQUENCY {
            carrier_frequency = MAX_FREQUENCY;
        }
        if formant_frequency_1 >= MAX_FREQUENCY {
            formant_frequency_1 = MAX_FREQUENCY;
        }
        if formant_frequency_2 >= MAX_FREQUENCY {
            formant_frequency_2 = MAX_FREQUENCY;
        }

        let mut f0_modulation =
            ParameterInterpolator::new(&mut self.carrier_frequency, carrier_frequency, out.len());
        let mut f1_modulation = ParameterInterpolator::new(
            &mut self.formant_1_frequency,
            formant_frequency_1,
            out.len(),
        );
        let mut f2_modulation = ParameterInterpolator::new(
            &mut self.formant_2_frequency,
            formant_frequency_2,
            out.len(),
        );
        let mut carrier_shape_modulation =
            ParameterInterpolator::new(&mut self.carrier_shape, carrier_shape, out.len());

        for out_sample in out.iter_mut() {
            let f0 = f0_modulation.next();
            let f1 = f1_modulation.next();
            let f2 = f2_modulation.next();

            self.carrier_phase += carrier_frequency;
            if self.carrier_phase >= 1.0 {
                self.carrier_phase -= 1.0;
                let reset_time = self.carrier_phase / f0;
                self.formant_1_phase = reset_time * f1;
                self.formant_2_phase = reset_time * f2;
            } else {
                self.formant_1_phase += f1;
                if self.formant_1_phase >= 1.0 {
                    self.formant_1_phase -= 1.0;
                }
                self.formant_2_phase += f2;
                if self.formant_2_phase >= 1.0 {
                    self.formant_2_phase -= 1.0;
                }
            }

            let carrier = sine(self.carrier_phase * 0.5 + 0.25) + 1.0;
            let reset_phase = 0.75 - 0.25 * carrier_shape_modulation.next();
            let reset_amplitude = sine(reset_phase);
            let formant_0 = sine(self.formant_1_phase + reset_phase) - reset_amplitude;
            let formant_1 = sine(self.formant_2_phase + reset_phase) - reset_amplitude;
            *out_sample = carrier * (formant_0 + formant_1) * 0.25 + reset_amplitude;
        }
    }
}
