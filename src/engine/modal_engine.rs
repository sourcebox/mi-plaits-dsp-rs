//! Modal resonator.
//!
//! Engine parameters:
//! - *HARMONICS:* amount of inharmonicity, or material selection.
//! - *TIMBRE:* excitation brightness and dust density.
//! - *MORPH:* decay time (energy absorption).
//!
//! *AUX* signal: raw exciter signal.
//!
//! When the *TRIG* input is not patched, the resonator is excited by dust (particle) noise.
//! Otherwise, the resonator is excited by a short burst of filtered white noise,
//! or by a low-pass filtered click.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use alloc::boxed::Box;
use alloc::vec;

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::physical_modelling::modal_voice::ModalVoice;
use crate::utils::{one_pole, scaled_smoothing_coefficient, REFERENCE_SAMPLE_RATE};

#[derive(Debug, Clone)]
pub struct ModalEngine {
    voice: ModalVoice,
    harmonics_lp: f32,

    temp_buffer_1: Box<[f32]>,
    temp_buffer_2: Box<[f32]>,

    // Sample rate dependent constants
    sample_rate_hz: f32,
    smoothing_coefficient: f32,
}

impl ModalEngine {
    pub fn new(block_size: usize) -> Self {
        Self {
            voice: ModalVoice::default(),
            harmonics_lp: 0.0,
            temp_buffer_1: vec![0.0; block_size].into_boxed_slice(),
            temp_buffer_2: vec![0.0; block_size].into_boxed_slice(),
            sample_rate_hz: 48000.0,
            smoothing_coefficient: 0.01,
        }
    }
}

impl Engine for ModalEngine {
    fn init(&mut self, sample_rate_hz: f32) {
        self.sample_rate_hz = sample_rate_hz;
        self.harmonics_lp = 0.0;
        // The harmonics parameter is smoothed once per block; keep the
        // smoothing time constant in seconds when the block rate changes
        // with the sample rate.
        self.smoothing_coefficient =
            scaled_smoothing_coefficient(0.01, REFERENCE_SAMPLE_RATE / sample_rate_hz);
        self.reset();
    }

    fn reset(&mut self) {
        self.voice.init(self.sample_rate_hz);
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        out.fill(0.0);
        aux.fill(0.0);

        one_pole(
            &mut self.harmonics_lp,
            parameters.harmonics,
            self.smoothing_coefficient,
        );

        let sustain = matches!(parameters.trigger, TriggerState::Unpatched);
        let trigger = matches!(parameters.trigger, TriggerState::RisingEdge);

        self.voice.render(
            sustain,
            trigger,
            parameters.accent,
            note_to_frequency(parameters.note, parameters.a0_normalized),
            self.harmonics_lp,
            parameters.timbre,
            parameters.morph,
            &mut self.temp_buffer_1,
            &mut self.temp_buffer_2,
            out,
            aux,
        );
    }
}
