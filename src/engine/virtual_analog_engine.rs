//! Pair of classic waveforms.
//!
//! Virtual-analog synthesis of classic waveforms using two variable shape oscillators
//! with sync and crossfading.
//!
//! Engine parameters:
//! - *HARMONICS:* detuning between the two waves.
//! - *TIMBRE:* variable square, from narrow pulse to full square to hardsync formants.
//! - *MORPH:* variable saw, from triangle to saw with an increasingly wide notch (*Braids’* CSAW).
//!
//! *AUX* signal: sum of two hardsync’ed waveforms, the shape of which is controlled by *MORPH*
//! and detuning by *HARMONICS*.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use alloc::boxed::Box;
use alloc::vec;

use super::{note_to_frequency, Engine, EngineParameters};
use crate::oscillator::variable_saw_oscillator::VariableSawOscillator;
use crate::oscillator::variable_shape_oscillator::VariableShapeOscillator;
use crate::utils::parameter_interpolator::ParameterInterpolator;

#[derive(Debug, Clone)]
pub struct VirtualAnalogEngine {
    primary: VariableShapeOscillator,
    auxiliary: VariableShapeOscillator,

    sync: VariableShapeOscillator,
    variable_saw: VariableSawOscillator,

    auxiliary_amount: f32,
    xmod_amount: f32,

    temp_buffer: Box<[f32]>,
}

impl VirtualAnalogEngine {
    pub fn new(block_size: usize) -> Self {
        Self {
            primary: VariableShapeOscillator::new(),
            auxiliary: VariableShapeOscillator::new(),
            sync: VariableShapeOscillator::new(),
            variable_saw: VariableSawOscillator::new(),
            auxiliary_amount: 0.0,
            xmod_amount: 0.0,
            temp_buffer: vec![0.0; block_size].into_boxed_slice(),
        }
    }
}

impl Engine for VirtualAnalogEngine {
    fn init(&mut self) {
        self.primary.init();
        self.auxiliary.init();
        self.auxiliary.set_master_phase(0.25);
        self.sync.init();
        self.variable_saw.init();
        self.auxiliary_amount = 0.0;
        self.xmod_amount = 0.0;
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        // VA_VARIANT 2
        // 1 = variable square controlled by TIMBRE.
        // 2 = variable saw controlled by MORPH.
        // OUT = 1 + 2.
        // AUX = dual variable waveshape controlled by MORPH, self sync by TIMBRE.

        let sync_amount = parameters.timbre * parameters.timbre;
        let auxiliary_detune = compute_detuning(parameters.harmonics);
        let primary_f = note_to_frequency(parameters.note);
        let auxiliary_f = note_to_frequency(parameters.note + auxiliary_detune);
        let primary_sync_f = note_to_frequency(parameters.note + sync_amount * 48.0);
        let auxiliary_sync_f =
            note_to_frequency(parameters.note + auxiliary_detune + sync_amount * 48.0);

        let mut shape = parameters.morph * 1.5;
        shape = shape.clamp(0.0, 1.0);

        let mut pw = 0.5 + (parameters.morph - 0.66) * 1.46;
        pw = pw.clamp(0.5, 0.995);

        // Render monster sync to AUX.
        self.primary
            .render(primary_f, primary_sync_f, pw, shape, 0.0, out, true, false);
        self.auxiliary.render(
            auxiliary_f,
            auxiliary_sync_f,
            pw,
            shape,
            0.0,
            aux,
            true,
            false,
        );
        for (aux_sample, out_sample) in aux.iter_mut().zip(out.iter()) {
            *aux_sample = (*aux_sample - *out_sample) * 0.5;
        }

        // Render double varishape to OUT.
        let mut square_pw = 1.3 * parameters.timbre - 0.15;
        square_pw = square_pw.clamp(0.005, 0.5);

        let square_sync_ratio = if parameters.timbre < 0.5 {
            0.0
        } else {
            (parameters.timbre - 0.5) * (parameters.timbre - 0.5) * 4.0 * 48.0
        };

        let square_gain = f32::min(parameters.timbre * 8.0, 1.0);

        let mut saw_pw = if parameters.morph < 0.5 {
            parameters.morph + 0.5
        } else {
            1.0 - (parameters.morph - 0.5) * 2.0
        };
        saw_pw *= 1.1;
        saw_pw = saw_pw.clamp(0.005, 1.0);

        let mut saw_shape = 10.0 - 21.0 * parameters.morph;
        saw_shape = saw_shape.clamp(0.0, 1.0);

        let mut saw_gain = 8.0 * (1.0 - parameters.morph);
        saw_gain = saw_gain.clamp(0.02, 1.0);

        let square_sync_f = note_to_frequency(parameters.note + square_sync_ratio);

        self.sync.render(
            primary_f,
            square_sync_f,
            square_pw,
            1.0,
            0.0,
            &mut self.temp_buffer,
            true,
            false,
        );
        self.variable_saw
            .render(auxiliary_f, saw_pw, saw_shape, out);

        let norm = 1.0 / (f32::max(square_gain, saw_gain));

        let mut square_gain_modulation = ParameterInterpolator::new(
            &mut self.auxiliary_amount,
            square_gain * 0.3 * norm,
            out.len(),
        );

        let mut saw_gain_modulation =
            ParameterInterpolator::new(&mut self.xmod_amount, saw_gain * 0.5 * norm, out.len());

        for (out_sample, temp_sample) in out.iter_mut().zip(self.temp_buffer.iter()) {
            *out_sample = *out_sample * saw_gain_modulation.next()
                + square_gain_modulation.next() * *temp_sample;
        }
    }
}

#[inline]
fn squash(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

const INTERVALS: [f32; 5] = [0.0, 7.01, 12.01, 19.01, 24.01];

fn compute_detuning(mut detune: f32) -> f32 {
    detune = 2.05 * detune - 1.025;
    detune = detune.clamp(-1.0, 1.0);

    let sign = if detune < 0.0 { -1.0 } else { 1.0 };
    detune = detune * sign * 3.9999;
    let detune_integral = detune as usize;
    let detune_fractional = detune - (detune_integral as f32);

    let a = INTERVALS[detune_integral];
    let b = INTERVALS[detune_integral + 1];

    (a + (b - a) * squash(squash(detune_fractional))) * sign
}
