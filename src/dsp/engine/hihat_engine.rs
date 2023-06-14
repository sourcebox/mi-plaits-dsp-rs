//! Analog hi-hat model.
//!
//! A bunch of square oscillators generating a harsh, metallic tone.
//!
//! Engine parameters:
//! - *HARMONICS:* balance of the metallic and filtered noise.
//! - *TIMBRE:* high-pass filter cutoff.
//! - *MORPH:* decay time.
//!
//! Outputs:
//! - *OUT* signal: 6 square oscillators and a dirty transistor VCA
//! - *AUX* signal: uses three pairs of square oscillators ringmodulating each other,
//!   and a clean, linear VCA

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::dsp::allocate_buffer;
use crate::dsp::drums::hihat::{Hihat, NoiseType, VcaType};

#[derive(Debug)]
pub struct HihatEngine<'a> {
    hi_hat_1: Hihat,
    hi_hat_2: Hihat,

    temp_buffer_1: &'a mut [f32],
    temp_buffer_2: &'a mut [f32],
}

impl<'a> HihatEngine<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            hi_hat_1: Hihat::default(),
            hi_hat_2: Hihat::default(),
            temp_buffer_1: allocate_buffer(buffer_allocator, block_size).unwrap(),
            temp_buffer_2: allocate_buffer(buffer_allocator, block_size).unwrap(),
        }
    }
}

impl<'a> Engine for HihatEngine<'a> {
    fn init(&mut self) {
        self.hi_hat_1.init();
        self.hi_hat_2.init();
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        let f0 = note_to_frequency(parameters.note);

        let sustain = matches!(parameters.trigger, TriggerState::Unpatched);
        let trigger = matches!(parameters.trigger, TriggerState::RisingEdge);

        self.hi_hat_1.render(
            sustain,
            trigger,
            parameters.accent,
            f0,
            parameters.timbre,
            parameters.morph,
            parameters.harmonics,
            self.temp_buffer_1,
            self.temp_buffer_2,
            out,
            NoiseType::Square,
            VcaType::Swing,
            true,
            false,
        );

        self.hi_hat_2.render(
            sustain,
            trigger,
            parameters.accent,
            f0,
            parameters.timbre,
            parameters.morph,
            parameters.harmonics,
            self.temp_buffer_1,
            self.temp_buffer_2,
            aux,
            NoiseType::RingMod,
            VcaType::Linear,
            false,
            true,
        );
    }
}
