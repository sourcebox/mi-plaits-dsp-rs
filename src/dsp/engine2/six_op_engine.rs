//! 6-operator FM synth with 32 presets.
//!
//! Engine parameters:
//! - *HARMONICS:* preset selection.
//! - *TIMBRE:* modulator(s) level.
//! - *MORPH:* envelope and modula�on stretching/time-travel.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

use crate::dsp::allocate_buffer;
use crate::dsp::engine::{note_to_frequency, Engine, EngineParameters};

const NUM_SIX_OP_VOICES: usize = 2;

#[derive(Debug)]
pub struct SixOpEngine<'a> {
    temp_buffer: &'a mut [f32],
    acc_buffer: &'a mut [f32],
}

impl<'a> SixOpEngine<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            temp_buffer: allocate_buffer(buffer_allocator, block_size * 4).unwrap(),
            acc_buffer: allocate_buffer(buffer_allocator, block_size * NUM_SIX_OP_VOICES).unwrap(),
        }
    }

    pub fn load_syx_bank(&mut self, bank: &[u8; 4096]) {
        // TODO
    }
}

impl<'a> Engine for SixOpEngine<'a> {
    fn init(&mut self) {
        // TODO
    }

    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        already_enveloped: &mut bool,
    ) {
    }
    // TODO
}
