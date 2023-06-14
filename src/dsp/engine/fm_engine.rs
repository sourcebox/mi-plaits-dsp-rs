//! Two operator FM.
//!
//! Two sine-wave oscillators modulating each other’s phase.
//!
//! Engine parameters:
//! - *HARMONICS:* frequency ratio.
//! - *TIMBRE:* modulation index.
//! - *MORPH:* feedback, in the form of operator 2 modulating its own phase (past 12 o’clock, rough!)
//!   or operator 1’s phase (before 12 o’clock, chaotic!).
//!
//! *AUX* signal: sub-oscillator.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters};
use crate::dsp::downsampler::Downsampler;
use crate::dsp::resources::{LUT_FM_FREQUENCY_QUANTIZER, LUT_SINE};
use crate::dsp::A0;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::{interpolate, one_pole};

const OVERSAMPLING: usize = 4;

#[derive(Debug, Default)]
pub struct FmEngine {
    carrier_phase: u32,
    modulator_phase: u32,
    sub_phase: u32,

    previous_carrier_frequency: f32,
    previous_modulator_frequency: f32,
    previous_amount: f32,
    previous_feedback: f32,
    previous_sample: f32,

    sub_fir: f32,
    carrier_fir: f32,
}

impl FmEngine {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Engine for FmEngine {
    fn init(&mut self) {
        self.carrier_phase = 0;
        self.modulator_phase = 0;
        self.sub_phase = 0;

        self.previous_carrier_frequency = A0;
        self.previous_modulator_frequency = A0;
        self.previous_amount = 0.0;
        self.previous_feedback = 0.0;
        self.previous_sample = 0.0;
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        // 4x oversampling
        let note = parameters.note - 24.0;

        let ratio = interpolate(&LUT_FM_FREQUENCY_QUANTIZER, parameters.harmonics, 128.0);

        let modulator_note = note + ratio;
        let mut target_modulator_frequency = note_to_frequency(modulator_note);
        target_modulator_frequency = target_modulator_frequency.clamp(0.0, 0.5);

        // Reduce the maximum FM index for high pitched notes, to prevent aliasing.
        let mut hf_taming = 1.0 - (modulator_note - 72.0) * 0.025;
        hf_taming = hf_taming.clamp(0.0, 1.0);
        hf_taming *= hf_taming;

        let mut carrier_frequency = ParameterInterpolator::new(
            &mut self.previous_carrier_frequency,
            note_to_frequency(note),
            out.len(),
        );
        let mut modulator_frequency = ParameterInterpolator::new(
            &mut self.previous_modulator_frequency,
            target_modulator_frequency,
            out.len(),
        );
        let mut amount_modulation = ParameterInterpolator::new(
            &mut self.previous_amount,
            2.0 * parameters.timbre * parameters.timbre * hf_taming,
            out.len(),
        );
        let mut feedback_modulation = ParameterInterpolator::new(
            &mut self.previous_feedback,
            2.0 * parameters.morph - 1.0,
            out.len(),
        );

        let mut carrier_downsampler = Downsampler::new(&mut self.carrier_fir);
        let mut sub_downsampler = Downsampler::new(&mut self.sub_fir);

        for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter_mut()) {
            let amount = amount_modulation.next();
            let feedback = feedback_modulation.next();
            let phase_feedback = if feedback < 0.0 {
                0.5 * feedback * feedback
            } else {
                0.0
            };
            let carrier_increment = (4294967296.0 * carrier_frequency.next()) as u32;
            let _modulator_frequency = modulator_frequency.next();

            for j in 0..OVERSAMPLING {
                self.modulator_phase = self.modulator_phase.wrapping_add(
                    (4294967296.0
                        * _modulator_frequency
                        * (1.0 + self.previous_sample * phase_feedback)) as u32,
                );
                self.carrier_phase = self.carrier_phase.wrapping_add(carrier_increment);
                self.sub_phase = self.sub_phase.wrapping_add(carrier_increment >> 1);
                let modulator_fb = if feedback > 0.0 {
                    0.25 * feedback * feedback
                } else {
                    0.0
                };
                let modulator = sine_pm(self.modulator_phase, modulator_fb * self.previous_sample);
                let carrier = sine_pm(self.carrier_phase, amount * modulator);
                let sub = sine_pm(self.sub_phase, amount * carrier * 0.25);
                one_pole(&mut self.previous_sample, carrier, 0.05);
                carrier_downsampler.accumulate(j, carrier);
                sub_downsampler.accumulate(j, sub);
            }

            *out_sample = carrier_downsampler.read();
            *aux_sample = sub_downsampler.read();
        }
    }
}

#[inline]
fn sine_pm(mut phase: u32, fm: f32) -> f32 {
    phase = phase.wrapping_add((((fm + 4.0) * 536870912.0) as u32) << 3);
    let integral = phase >> 22;
    let fractional = (phase << 10) as f32 / 4294967296.0;
    let a = LUT_SINE[integral as usize];
    let b = LUT_SINE[integral as usize + 1];

    a + (b - a) * fractional
}
