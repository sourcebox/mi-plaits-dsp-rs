//! Phase distortion and phase modulation with an asymmetric triangle as the modulator.
//!
//! Engine parameters:
//! - *HARMONICS:* distortion frequency.
//! - *TIMBRE:* distortion amount.
//! - *MORPH:* distortion asymmetry.
//!
//! *OUT* signal: carrier is sync'ed (phase distortion).
//! *AUX* signal: carrier is free-running (phase modulation)).

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use alloc::boxed::Box;
use alloc::vec;

use crate::engine::{note_to_frequency, Engine, EngineParameters};
use crate::oscillator::sine_oscillator::sine;
use crate::oscillator::variable_shape_oscillator::VariableShapeOscillator;
use crate::resources::fm::LUT_FM_FREQUENCY_QUANTIZER;
use crate::utils::interpolate;
use crate::utils::units::semitones_to_ratio;

#[derive(Debug, Clone)]
pub struct PhaseDistortionEngine {
    shaper: VariableShapeOscillator,
    modulator: VariableShapeOscillator,

    temp_buffer_1: Box<[f32]>,
    temp_buffer_2: Box<[f32]>,
}

impl PhaseDistortionEngine {
    pub fn new(block_size: usize) -> Self {
        Self {
            shaper: VariableShapeOscillator::new(),
            modulator: VariableShapeOscillator::new(),
            temp_buffer_1: vec![0.0; block_size * 2].into_boxed_slice(),
            temp_buffer_2: vec![0.0; block_size * 2].into_boxed_slice(),
        }
    }
}

impl Engine for PhaseDistortionEngine {
    fn init(&mut self, _sample_rate_hz: f32) {
        self.shaper.init();
        self.modulator.init();
    }

    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        let f0 = 0.5 * note_to_frequency(parameters.note, parameters.a0_normalized);
        let modulator_f = f32::min(
            0.25,
            f0 * semitones_to_ratio(interpolate(
                &LUT_FM_FREQUENCY_QUANTIZER,
                parameters.harmonics,
                128.0,
            )),
        );
        let pw = 0.5 + parameters.morph * 0.49;
        let amount = 8.0 * parameters.timbre * parameters.timbre * (1.0 - modulator_f * 3.8);

        // Upsample by 2x
        let synced = &mut self.temp_buffer_1;
        let free_running = &mut self.temp_buffer_2;
        self.shaper
            .render(f0, modulator_f, pw, 0.0, amount, synced, true, true);
        self.modulator
            .render(f0, modulator_f, pw, 0.0, amount, free_running, false, true);

        for (n, (out_sample, aux_sample)) in out.iter_mut().zip(aux.iter_mut()).enumerate() {
            // Naive 0.5x downsampling.
            *out_sample = 0.5 * sine(synced[n * 2] + 0.25);
            *out_sample += 0.5 * sine(synced[n * 2 + 1] + 0.25);

            *aux_sample = 0.5 * sine(free_running[n * 2] + 0.25);
            *aux_sample += 0.5 * sine(free_running[n * 2 + 1] + 0.25);
        }
    }
}
