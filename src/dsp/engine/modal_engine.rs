//! One voice of modal synthesis.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::dsp::allocate_buffer;
use crate::dsp::physical_modelling::modal_voice::ModalVoice;
use crate::stmlib::dsp::one_pole;

#[derive(Debug)]
pub struct ModalEngine<'a> {
    voice: ModalVoice,
    harmonics_lp: f32,

    temp_buffer_1: &'a mut [f32],
    temp_buffer_2: &'a mut [f32],
}

impl<'a> Engine for ModalEngine<'a> {
    fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            voice: ModalVoice::default(),
            harmonics_lp: 0.0,
            temp_buffer_1: allocate_buffer(buffer_allocator, block_size),
            temp_buffer_2: allocate_buffer(buffer_allocator, block_size),
        }
    }

    fn init(&mut self) {
        self.harmonics_lp = 0.0;
        self.reset();
    }

    fn reset(&mut self) {
        self.voice.init();
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

        one_pole(&mut self.harmonics_lp, parameters.harmonics, 0.01);

        let sustain = matches!(
            parameters.trigger,
            TriggerState::Unpatched | TriggerState::UnpatchedAutotriggered
        );

        let trigger = matches!(
            parameters.trigger,
            TriggerState::RisingEdge | TriggerState::UnpatchedAutotriggered
        );

        self.voice.render(
            sustain,
            trigger,
            parameters.accent,
            note_to_frequency(parameters.note),
            self.harmonics_lp,
            parameters.timbre,
            parameters.morph,
            self.temp_buffer_1,
            self.temp_buffer_2,
            out,
            aux,
        );
    }
}
