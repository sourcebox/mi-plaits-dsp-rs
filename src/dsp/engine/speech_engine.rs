//! Vowel and speech synthesis.
//!
//! A collection of speech synthesis algorithms.
//!
//! Engine parameters:
//! - *HARMONICS:* crossfades between formant filtering, SAM, and LPC vowels, then goes
//!   through several banks of LPC words.
//! - *TIMBRE:* species selection, from Daleks to chipmunks. How does it work?
//!   This parameter either shifts the formants up or down independently of the pitch;
//!   or underclocks/overclocks the emulated LPC chip (with appropriate compensation
//!   to keep the pitch unchanged).
//! - *MORPH:* phoneme or word segment selection. When *HARMONICS* is past 11 o’clock,
//!   a list of words can be scanned through by turning the *MORPH* knob or by sending a CV
//!   to the corresponding input. One can also patch the trigger input to trigger the
//!   utterance of a word, use the FM attenuverter to control the intonation and the
//!   *MORPH* attenuverter to control speed.
//!
//! *AUX* signal: unfiltered vocal cords’ signal.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::dsp::allocate_buffer;
use crate::dsp::speech::lpc_speech_synth_controller::LpcSpeechSynthController;
use crate::dsp::speech::lpc_speech_synth_words::NUM_WORD_BANKS;
use crate::dsp::speech::naive_speech_synth::NaiveSpeechSynth;
use crate::dsp::speech::sam_speech_synth::SamSpeechSynth;
use crate::stmlib::dsp::hysteresis_quantizer::HysteresisQuantizer;

#[derive(Debug)]
pub struct SpeechEngine<'a> {
    word_bank_quantizer: HysteresisQuantizer,

    naive_speech_synth: NaiveSpeechSynth,
    sam_speech_synth: SamSpeechSynth,

    lpc_speech_synth_controller: LpcSpeechSynthController<'a>,
    temp_buffer_1: &'a mut [f32],
    temp_buffer_2: &'a mut [f32],
    prosody_amount: f32,
    speed: f32,
}

impl<'a> SpeechEngine<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            word_bank_quantizer: HysteresisQuantizer::new(),
            naive_speech_synth: NaiveSpeechSynth::new(),
            sam_speech_synth: SamSpeechSynth::new(),
            lpc_speech_synth_controller: LpcSpeechSynthController::new(buffer_allocator),
            temp_buffer_1: allocate_buffer(buffer_allocator, block_size),
            temp_buffer_2: allocate_buffer(buffer_allocator, block_size),
            prosody_amount: 0.0,
            speed: 1.0,
        }
    }
}

impl<'a> Engine for SpeechEngine<'a> {
    fn init(&mut self) {
        self.sam_speech_synth.init();
        self.naive_speech_synth.init();
        self.lpc_speech_synth_controller.init();
        self.word_bank_quantizer.init();
        self.prosody_amount = 0.0;
        self.speed = 0.0;
        self.reset();
    }

    fn reset(&mut self) {
        self.lpc_speech_synth_controller.reset();
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        already_enveloped: &mut bool,
    ) {
        let f0 = note_to_frequency(parameters.note);

        let group = parameters.harmonics * 6.0;

        let sustain = matches!(
            parameters.trigger,
            TriggerState::Unpatched | TriggerState::UnpatchedAutotriggered
        );

        let trigger = matches!(
            parameters.trigger,
            TriggerState::RisingEdge | TriggerState::UnpatchedAutotriggered
        );

        // Interpolates between the 3 models: naive, SAM, LPC.
        if group <= 2.0 {
            *already_enveloped = false;

            let mut blend = group;

            if group <= 1.0 {
                self.naive_speech_synth.render(
                    trigger,
                    f0,
                    parameters.morph,
                    parameters.timbre,
                    self.temp_buffer_1,
                    aux,
                    out,
                );
            } else {
                self.lpc_speech_synth_controller.render(
                    sustain,
                    trigger,
                    -1,
                    f0,
                    0.0,
                    0.0,
                    parameters.morph,
                    parameters.timbre,
                    1.0,
                    aux,
                    out,
                );
                blend = 2.0 - blend;
            }

            self.sam_speech_synth.render(
                sustain,
                f0,
                parameters.morph,
                parameters.timbre,
                self.temp_buffer_1,
                self.temp_buffer_2,
            );

            blend = blend * blend * (3.0 - 2.0 * blend);
            blend = blend * blend * (3.0 - 2.0 * blend);

            for (i, (out_sample, aux_sample)) in out.iter_mut().zip(aux.iter_mut()).enumerate() {
                *aux_sample += (self.temp_buffer_1[i] - *aux_sample) * blend;
                *out_sample += (self.temp_buffer_2[i] - *out_sample) * blend;
            }
        } else {
            // Change phonemes/words for LPC.
            let word_bank = self
                .word_bank_quantizer
                .process_with_default((group - 2.0) * 0.275, NUM_WORD_BANKS + 1)
                - 1;

            let replay_prosody = word_bank >= 0 && !trigger;

            *already_enveloped = replay_prosody;

            self.lpc_speech_synth_controller.render(
                sustain,
                trigger,
                word_bank,
                f0,
                self.prosody_amount,
                self.speed,
                parameters.morph,
                parameters.timbre,
                if replay_prosody {
                    parameters.accent
                } else {
                    1.0
                },
                aux,
                out,
            );
        }
    }
}

impl<'a> SpeechEngine<'a> {
    pub fn set_prosody_amount(&mut self, prosody_amount: f32) {
        self.prosody_amount = prosody_amount;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }
}
