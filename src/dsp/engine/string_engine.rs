//! Inharmonic string modeling.
//!
//! Engine parameters:
//! - *HARMONICS:* amount of inharmonicity, or material selection.
//! - *TIMBRE:* excitation brightness and dust density.
//! - *MORPH:* decay time (energy absorption).
//!
//! *AUX* signal: raw exciter signal.
//!
//! When the *TRIG* input is not patched, the string is excited by dust (particle) noise.
//! Otherwise, the string is excited by a short burst of filtered white noise,
//! or by a low-pass filtered click.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use alloc::boxed::Box;
use alloc::vec;

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::dsp::physical_modelling::string_voice::StringVoice;
use crate::stmlib::dsp::delay_line::DelayLine;

const NUM_STRINGS: usize = 3;

#[derive(Debug)]
pub struct StringEngine {
    voice: [StringVoice; NUM_STRINGS],

    f0: [f32; NUM_STRINGS],
    f0_delay: DelayLine<f32, 16>,
    active_string: usize,

    temp_buffer_1: Box<[f32]>,
    temp_buffer_2: Box<[f32]>,
}

impl StringEngine {
    pub fn new(block_size: usize) -> Self {
        Self {
            voice: core::array::from_fn(|_| StringVoice::new()),
            f0: [0.0; NUM_STRINGS],
            f0_delay: DelayLine::<f32, 16>::new(),
            active_string: 0,
            temp_buffer_1: vec![0.0; block_size].into_boxed_slice(),
            temp_buffer_2: vec![0.0; block_size].into_boxed_slice(),
        }
    }
}

impl Engine for StringEngine {
    fn init(&mut self) {
        for voice in &mut self.voice {
            voice.init();
        }
        self.f0 = [0.0; NUM_STRINGS];
        self.active_string = NUM_STRINGS - 1;
        self.reset();
    }

    fn reset(&mut self) {
        self.f0_delay.reset();
        for voice in &mut self.voice {
            voice.reset();
        }
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        let sustain = matches!(parameters.trigger, TriggerState::Unpatched);
        let trigger = matches!(parameters.trigger, TriggerState::RisingEdge);

        if trigger {
            // 8 in original firmware version.
            // 05.01.18: mic.w: problem with microbrute.
            self.f0[self.active_string] = self.f0_delay.read_with_delay(14);
            self.active_string = (self.active_string + 1) % NUM_STRINGS;
        }

        let f0 = note_to_frequency(parameters.note);
        self.f0[self.active_string] = f0;
        self.f0_delay.write(f0);

        out.fill(0.0);
        aux.fill(0.0);

        for i in 0..NUM_STRINGS {
            self.voice[i].render(
                sustain && i == self.active_string,
                trigger && i == self.active_string,
                parameters.accent,
                self.f0[i],
                parameters.harmonics,
                parameters.timbre * parameters.timbre,
                parameters.morph,
                &mut self.temp_buffer_1,
                &mut self.temp_buffer_2,
                out,
                aux,
            );
        }
    }
}
