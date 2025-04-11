//! Sinewave with aliasing-free phase reset.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::oscillator::oscillator::MAX_FREQUENCY;
use crate::oscillator::sine_oscillator::sine;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::polyblep::this_blep_sample;

#[derive(Debug, Default)]
pub struct FormantOscillator {
    // Oscillator state.
    carrier_phase: f32,
    formant_phase: f32,
    next_sample: f32,

    // For interpolation of parameters.
    carrier_frequency: f32,
    formant_frequency: f32,
    phase_shift: f32,
}

impl FormantOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.carrier_phase = 0.0;
        self.formant_phase = 0.0;
        self.next_sample = 0.0;

        self.carrier_frequency = 0.0;
        self.formant_frequency = 0.01;
        self.phase_shift = 0.0;
    }

    #[inline]
    pub fn render(
        &mut self,
        mut carrier_frequency: f32,
        mut formant_frequency: f32,
        phase_shift: f32,
        out: &mut [f32],
    ) {
        if carrier_frequency >= MAX_FREQUENCY {
            carrier_frequency = MAX_FREQUENCY;
        }
        if formant_frequency >= MAX_FREQUENCY {
            formant_frequency = MAX_FREQUENCY;
        }

        let mut carrier_fm =
            ParameterInterpolator::new(&mut self.carrier_frequency, carrier_frequency, out.len());
        let mut formant_fm =
            ParameterInterpolator::new(&mut self.formant_frequency, formant_frequency, out.len());
        let mut pm = ParameterInterpolator::new(&mut self.phase_shift, phase_shift, out.len());

        let mut next_sample = self.next_sample;

        for out_sample in out.iter_mut() {
            let mut this_sample = next_sample;
            next_sample = 0.0;

            let carrier_frequency = carrier_fm.next();
            let formant_frequency = formant_fm.next();

            self.carrier_phase += carrier_frequency;

            if self.carrier_phase >= 1.0 {
                self.carrier_phase -= 1.0;
                let reset_time = self.carrier_phase / carrier_frequency;

                let formant_phase_at_reset =
                    self.formant_phase + (1.0 - reset_time) * formant_frequency;
                let before = sine(formant_phase_at_reset + pm.subsample(1.0 - reset_time));
                let after = sine(0.0 + pm.subsample(1.0));
                let discontinuity = after - before;
                this_sample += discontinuity * this_blep_sample(reset_time);
                next_sample += discontinuity * this_blep_sample(reset_time);
                self.formant_phase = reset_time * formant_frequency;
            } else {
                self.formant_phase += formant_frequency;
                if self.formant_phase >= 1.0 {
                    self.formant_phase -= 1.0;
                }
            }

            let phase_shift = pm.next();
            next_sample += sine(self.formant_phase + phase_shift);

            *out_sample = this_sample;
        }

        self.next_sample = next_sample;
    }
}
