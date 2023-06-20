//! 6-operator FM synth with 32 presets.
//!
//! Engine parameters:
//! - *HARMONICS:* preset selection.
//! - *TIMBRE:* modulator(s) level.
//! - *MORPH:* envelope and modula�on stretching/time-travel.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

use crate::dsp::engine::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::dsp::fm::{
    algorithms::Algorithms,
    lfo::Lfo,
    patch::{Patch, SYX_SIZE},
    voice::{Voice, VoiceParameters},
};
use crate::dsp::{allocate_buffer, SAMPLE_RATE};
use crate::stmlib::dsp::hysteresis_quantizer::HysteresisQuantizer2;
use crate::stmlib::dsp::soft_clip;

const NUM_SIX_OP_VOICES: usize = 2;
const NUM_PATCHES_PER_BANK: usize = 32;

#[derive(Debug)]
pub struct SixOpEngine<'a> {
    patch_index_quantizer: HysteresisQuantizer2,
    algorithms: Algorithms<6, 32>,
    patches: [Patch; NUM_PATCHES_PER_BANK],
    voice: [FmVoice<'a>; NUM_SIX_OP_VOICES],

    temp_buffer_1: &'a mut [f32],
    temp_buffer_2: &'a mut [f32],
    temp_buffer_3: &'a mut [f32],
    temp_buffer_4: &'a mut [f32],
    acc_buffer: &'a mut [f32],

    active_voice: i32,
    rendered_voice: i32,
}

impl<'a> SixOpEngine<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            patch_index_quantizer: HysteresisQuantizer2::new(),
            algorithms: Algorithms::<6, 32>::new(),
            patches: core::array::from_fn(|_| Patch::new()),
            voice: core::array::from_fn(|_| FmVoice::new(buffer_allocator, block_size)),
            temp_buffer_1: allocate_buffer(buffer_allocator, block_size).unwrap(),
            temp_buffer_2: allocate_buffer(buffer_allocator, block_size).unwrap(),
            temp_buffer_3: allocate_buffer(buffer_allocator, block_size).unwrap(),
            temp_buffer_4: allocate_buffer(buffer_allocator, block_size).unwrap(),
            acc_buffer: allocate_buffer(buffer_allocator, block_size * NUM_SIX_OP_VOICES).unwrap(),
            active_voice: 0,
            rendered_voice: 0,
        }
    }

    pub fn load_syx_bank(&mut self, bank: &[u8; 4096]) {
        for (i, patch) in self.patches.iter_mut().enumerate() {
            (*patch).unpack(&bank[i * SYX_SIZE..]);
        }
    }
}

impl<'a> Engine for SixOpEngine<'a> {
    fn init(&mut self) {
        self.patch_index_quantizer.init(32, 0.005, false);

        self.algorithms.init();

        for voice in self.voice.iter_mut() {
            // TODO: fix lifetime issues
            // voice.init(&self.algorithms, SAMPLE_RATE);
        }

        self.active_voice = (NUM_SIX_OP_VOICES - 1) as i32;
        self.rendered_voice = 0;
    }

    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        // TODO: remove after fixes
        return;

        let patch_index = self
            .patch_index_quantizer
            .process(parameters.harmonics * 1.02);

        if parameters.trigger == TriggerState::Unpatched {
            let t = parameters.morph;
            self.voice[0].mutable_lfo().scrub(2.0 * SAMPLE_RATE * t);

            for (i, voice) in self.voice.iter_mut().enumerate() {
                // TODO: fix lifetime issues
                // voice.load_patch(Some(&self.patches[patch_index as usize]));
                let p = voice.mutable_parameters();
                p.sustain = if i == 0 { true } else { false };
                p.gate = false;
                p.note = parameters.note;
                p.velocity = parameters.accent;
                p.brightness = parameters.timbre;
                p.envelope_control = t;
                // TODO: fix mutability issues
                // voice.set_modulations(self.voice[0].lfo());
            }
        } else {
            if parameters.trigger == TriggerState::RisingEdge {
                self.active_voice = (self.active_voice + 1) % NUM_SIX_OP_VOICES as i32;
                // TODO: fix lifetime issues
                // self.voice[self.active_voice as usize]
                //     .load_patch(Some(&self.patches[patch_index as usize]));
                self.voice[self.active_voice as usize].mutable_lfo().reset();
            }
            let p = self.voice[self.active_voice as usize].mutable_parameters();
            p.note = parameters.note;
            p.velocity = parameters.accent;
            p.envelope_control = parameters.morph;
            self.voice[self.active_voice as usize]
                .mutable_lfo()
                .step(out.len() as f32);

            for (i, voice) in self.voice.iter_mut().enumerate() {
                let p = voice.mutable_parameters();
                p.brightness = parameters.timbre;
                p.sustain = false;
                p.gate =
                    (parameters.trigger == TriggerState::High) && (i == self.active_voice as usize);
                // TODO: fix mutability issues
                // if voice.patch() != self.voice[self.active_voice as usize].patch() {
                //     voice.mutable_lfo().step(out.len() as f32);
                //     voice.set_modulations(voice.lfo());
                // } else {
                //     voice.set_modulations(self.voice[self.active_voice as usize].lfo());
                // }
            }
        }

        // Naive block rendering.
        // fill(temp_buffer_[0], temp_buffer_[size], 0.0f);
        // for (int i = 0; i < kNumSixOpVoices; ++i) {
        //   voice_[i].Render(temp_buffer_, size);
        // }

        // Staggered rendering.
        self.temp_buffer_1.copy_from_slice(self.acc_buffer);
        self.temp_buffer_2.fill(0.0);
        self.rendered_voice = (self.rendered_voice + 1) % NUM_SIX_OP_VOICES as i32;

        let mut buffers = [
            self.temp_buffer_1.as_mut(),
            self.temp_buffer_2.as_mut(),
            self.temp_buffer_3.as_mut(),
            self.temp_buffer_4.as_mut(),
        ];
        self.voice[self.rendered_voice as usize].render(&mut buffers);

        for (i, (out_sample, aux_sample)) in out.iter_mut().zip(aux.iter_mut()).enumerate() {
            *out_sample = soft_clip(self.temp_buffer_1[i] * 0.25);
            *aux_sample = *out_sample;
        }

        self.acc_buffer.copy_from_slice(self.temp_buffer_2);
    }
}

#[derive(Debug)]
pub struct FmVoice<'a> {
    patch: Option<&'a Patch>,

    lfo: Lfo,
    voice: Voice<'a, 6, 32>,
    parameters: VoiceParameters,
}

impl<'a> FmVoice<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            patch: None,
            lfo: Lfo::new(),
            voice: Voice::<'a, 6, 32>::new(buffer_allocator, block_size),
            parameters: VoiceParameters::new(),
        }
    }

    pub fn init(&mut self, algorithms: &'a Algorithms<6, 32>, sample_rate: f32) {
        self.voice.init(algorithms, sample_rate);
        self.lfo.init(sample_rate);
        self.parameters.sustain = false;
        self.parameters.gate = false;
        self.parameters.note = 48.0;
        self.parameters.velocity = 0.5;
        self.parameters.brightness = 0.5;
        self.parameters.envelope_control = 0.5;
        self.parameters.pitch_mod = 0.0;
        self.parameters.amp_mod = 0.0;

        self.patch = None;
    }

    pub fn load_patch(&mut self, patch: Option<&'a Patch>) {
        if patch == self.patch {
            return;
        }

        self.patch = patch;
        self.voice.set_patch(self.patch);

        if let Some(patch) = patch {
            self.lfo.set(&patch.modulations);
        }
    }

    #[inline]
    pub fn render(&mut self, buffer: &mut [&mut [f32]; 4]) {
        if self.patch.is_none() {
            return;
        }

        self.voice.render(&self.parameters, buffer);
    }

    #[inline]
    pub fn unload_patch(&mut self) {
        self.patch = None;
    }

    #[inline]
    pub fn patch(&self) -> Option<&Patch> {
        self.patch
    }

    #[inline]
    pub fn mutable_parameters(&mut self) -> &mut VoiceParameters {
        &mut self.parameters
    }

    #[inline]
    pub fn mutable_lfo(&mut self) -> &mut Lfo {
        &mut self.lfo
    }

    #[inline]
    pub fn lfo(&self) -> &Lfo {
        &self.lfo
    }

    #[inline]
    pub fn set_modulations(&mut self, lfo: &Lfo) {
        self.parameters.pitch_mod = lfo.pitch_mod();
        self.parameters.amp_mod = lfo.amp_mod();
    }
}