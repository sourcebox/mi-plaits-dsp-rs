//! Virtual analog oscillator with VCF.
//!
//! Engine parameters:
//! - *HARMONICS:* resonance and filter character - gentle 24dB/octave 0.0, harsh 12dB/octave 1.0.
//! - *TIMBRE:* filter cutoff.
//! - *MORPH:* waveform and sub level.
//!
//! *OUT* signal: LP output.
//! *AUX* signal: 12dB/octave HP output.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use crate::engine::{note_to_frequency, Engine, EngineParameters};
use crate::oscillator::variable_shape_oscillator::VariableShapeOscillator;
use crate::utils::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::soft_clip;
use crate::utils::units::semitones_to_ratio;

#[derive(Debug, Default, Clone)]
pub struct VirtualAnalogVcfEngine {
    svf: [Svf; 2],
    oscillator: VariableShapeOscillator,
    sub_oscillator: VariableShapeOscillator,

    previous_cutoff: f32,
    previous_stage2_gain: f32,
    previous_q: f32,
    previous_gain: f32,
    previous_sub_gain: f32,
}

impl VirtualAnalogVcfEngine {
    pub fn new() -> Self {
        Self {
            svf: [Svf::new(), Svf::new()],
            oscillator: VariableShapeOscillator::new(),
            sub_oscillator: VariableShapeOscillator::new(),

            previous_cutoff: 0.0,
            previous_stage2_gain: 0.0,
            previous_q: 0.0,
            previous_gain: 0.0,
            previous_sub_gain: 0.0,
        }
    }
}

impl Engine for VirtualAnalogVcfEngine {
    fn init(&mut self) {
        self.oscillator.init();
        self.sub_oscillator.init();

        self.svf[0].init();
        self.svf[1].init();

        self.previous_sub_gain = 0.0;
        self.previous_cutoff = 0.0;
        self.previous_stage2_gain = 0.0;
        self.previous_q = 0.0;
        self.previous_gain = 0.0;
    }

    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        let f0 = note_to_frequency(parameters.note);

        let shape = ((parameters.morph - 0.25) * 2.0 + 0.5).clamp(0.5, 1.0);

        let mut pw = (parameters.morph - 0.5) * 2.0 + 0.5;
        if parameters.morph > 0.75 {
            pw = 2.5 - parameters.morph * 2.0;
        }
        let pw = pw.clamp(0.5, 0.98);

        let sub_gain = f32::max(f32::abs(parameters.morph - 0.5) - 0.3, 0.0) * 5.0;

        self.oscillator
            .render(0.0, f0, pw, shape, 0.0, out, false, false);
        self.sub_oscillator
            .render(0.0, f0 * 0.501, 0.5, 1.0, 0.0, aux, false, false);

        let cutoff = f0 * semitones_to_ratio((parameters.timbre - 0.2) * 120.0);

        let stage2_gain = 1.0 - ((parameters.harmonics - 0.4) * 4.0).clamp(0.0, 1.0);

        let resonance = 2.667 * f32::max(f32::abs(parameters.harmonics - 0.5) - 0.125, 0.0);
        let resonance_sqr = resonance * resonance;
        let q = resonance_sqr * resonance_sqr * 48.0;
        let gain = ((parameters.harmonics - 0.7) + 0.85).clamp(0.7 - resonance_sqr * 0.3, 1.0);

        let mut sub_gain_modulation =
            ParameterInterpolator::new(&mut self.previous_sub_gain, sub_gain, out.len());
        let mut cutoff_modulation =
            ParameterInterpolator::new(&mut self.previous_cutoff, cutoff, out.len());
        let mut stage2_gain_modulation =
            ParameterInterpolator::new(&mut self.previous_stage2_gain, stage2_gain, out.len());
        let mut q_modulation = ParameterInterpolator::new(&mut self.previous_q, q, out.len());
        let mut gain_modulation =
            ParameterInterpolator::new(&mut self.previous_gain, gain, out.len());

        for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter_mut()) {
            let cutoff = f32::min(cutoff_modulation.next(), 0.25);
            let q = q_modulation.next();
            let stage2_gain = stage2_gain_modulation.next();

            self.svf[0].set_f_q(cutoff, 0.5 + q, FrequencyApproximation::Fast);
            self.svf[1].set_f_q(cutoff, 0.5 + 0.025 * q, FrequencyApproximation::Fast);

            let gain = gain_modulation.next();
            let input = soft_clip((*out_sample + *aux_sample * sub_gain_modulation.next()) * gain);

            let mut lp = 0.0;
            let mut hp = 0.0;

            self.svf[0].process_dual(
                input,
                &mut lp,
                &mut hp,
                FilterMode::LowPass,
                FilterMode::HighPass,
            );

            lp = soft_clip(lp * gain);
            lp += stage2_gain * (soft_clip(self.svf[1].process(lp, FilterMode::LowPass)) - lp);

            *out_sample = lp;
            *aux_sample = soft_clip(hp * gain);
        }
    }
}
