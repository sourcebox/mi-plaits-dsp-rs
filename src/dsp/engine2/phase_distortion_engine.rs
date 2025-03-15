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

use core::alloc::GlobalAlloc;

use crate::dsp::allocate_buffer;
use crate::dsp::engine::{note_to_frequency, Engine, EngineParameters};
use crate::dsp::oscillator::sine_oscillator::sine;
use crate::dsp::oscillator::variable_shape_oscillator::VariableShapeOscillator;
use crate::dsp::resources::fm::LUT_FM_FREQUENCY_QUANTIZER;
use crate::stmlib::dsp::interpolate;
use crate::stmlib::dsp::units::semitones_to_ratio;

#[derive(Debug)]
pub struct PhaseDistortionEngine<'a> {
    shaper: VariableShapeOscillator,
    modulator: VariableShapeOscillator,
    temp_buffer_1: &'a mut [f32],
    temp_buffer_2: &'a mut [f32],
}

impl PhaseDistortionEngine<'_> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            shaper: VariableShapeOscillator::new(),
            modulator: VariableShapeOscillator::new(),
            temp_buffer_1: allocate_buffer(buffer_allocator, block_size * 2).unwrap(),
            temp_buffer_2: allocate_buffer(buffer_allocator, block_size * 2).unwrap(),
        }
    }
}

impl Engine for PhaseDistortionEngine<'_> {
    fn init(&mut self) {
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
        let f0 = 0.5 * note_to_frequency(parameters.note);
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
