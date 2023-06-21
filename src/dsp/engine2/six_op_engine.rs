//! 6-operator FM synth with 32 presets.
//!
//! Engine parameters:
//! - *HARMONICS:* preset selection.
//! - *TIMBRE:* modulator(s) level.
//! - *MORPH:* envelope and modula�on stretching/time-travel.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

use crate::dsp::engine::{Engine, EngineParameters, TriggerState};
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

static mut ALGORITHMS: Option<Algorithms<6, 32>> = None;

static mut PATCHES: Option<[Patch; NUM_PATCHES_PER_BANK]> = None;

#[derive(Debug)]
pub struct SixOpEngine<'a> {
    patch_index_quantizer: HysteresisQuantizer2,
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
    pub fn new<A: GlobalAlloc>(buffer_allocator: &A, block_size: usize) -> Self {
        unsafe {
            let mut algorithms = Algorithms::<6, 32>::new();
            algorithms.init();
            ALGORITHMS = Some(algorithms);
            let patches: [Patch; NUM_PATCHES_PER_BANK] = core::array::from_fn(|_| Patch::new());
            PATCHES = Some(patches);
        }
        Self {
            patch_index_quantizer: HysteresisQuantizer2::new(),
            voice: core::array::from_fn(|_| FmVoice::new(buffer_allocator, block_size)),
            temp_buffer_1: allocate_buffer(buffer_allocator, block_size).unwrap(),
            temp_buffer_2: allocate_buffer(buffer_allocator, block_size).unwrap(),
            temp_buffer_3: allocate_buffer(buffer_allocator, block_size).unwrap(),
            temp_buffer_4: allocate_buffer(buffer_allocator, block_size).unwrap(),
            acc_buffer: allocate_buffer(buffer_allocator, block_size).unwrap(),
            active_voice: 0,
            rendered_voice: 0,
        }
    }

    pub fn load_syx_bank(&mut self, bank: &[u8; 4096]) {
        let patches = unsafe { PATCHES.as_mut().unwrap() };
        for (i, patch) in patches.iter_mut().enumerate() {
            (*patch).unpack(&bank[i * SYX_SIZE..]);
        }
    }
}

impl<'a> Engine for SixOpEngine<'a> {
    fn init(&mut self) {
        self.patch_index_quantizer.init(32, 0.005, false);

        for voice in self.voice.iter_mut() {
            unsafe {
                voice.init(ALGORITHMS.as_ref().unwrap(), SAMPLE_RATE);
            }
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
        let patch_index = self
            .patch_index_quantizer
            .process(parameters.harmonics * 1.02);

        if parameters.trigger == TriggerState::Unpatched {
            let t = parameters.morph;
            self.voice[0].mutable_lfo().scrub(2.0 * SAMPLE_RATE * t);

            let pitch_mod = self.voice[0].lfo().pitch_mod();
            let amp_mod = self.voice[0].lfo().amp_mod();

            for (i, voice) in self.voice.iter_mut().enumerate() {
                let patches = unsafe { PATCHES.as_ref().unwrap() };
                voice.load_patch(Some(&patches[patch_index as usize]));
                let p = voice.mutable_parameters();
                p.sustain = i == 0;
                p.gate = false;
                p.note = parameters.note;
                p.velocity = parameters.accent;
                p.brightness = parameters.timbre;
                p.envelope_control = t;
                voice.set_modulations(pitch_mod, amp_mod);
            }
        } else {
            if parameters.trigger == TriggerState::RisingEdge {
                self.active_voice = (self.active_voice + 1) % NUM_SIX_OP_VOICES as i32;
                let patches = unsafe { PATCHES.as_ref().unwrap() };
                self.voice[self.active_voice as usize]
                    .load_patch(Some(&patches[patch_index as usize]));
                self.voice[self.active_voice as usize].mutable_lfo().reset();
            }
            let p = self.voice[self.active_voice as usize].mutable_parameters();
            p.note = parameters.note;
            p.velocity = parameters.accent;
            p.envelope_control = parameters.morph;
            self.voice[self.active_voice as usize]
                .mutable_lfo()
                .step(out.len() as f32);

            let active_voice_lfo = self.voice[self.active_voice as usize].lfo();
            let active_voice_pitch_mod = active_voice_lfo.pitch_mod();
            let active_voice_amp_mod = active_voice_lfo.amp_mod();
            let active_voice_patch = self.voice[self.active_voice as usize].patch();

            let mut voice_patch_changed = [false; NUM_SIX_OP_VOICES];

            for (i, voice) in self.voice.iter().enumerate() {
                if voice.patch() != active_voice_patch {
                    voice_patch_changed[i] = true;
                }
            }

            for (i, voice) in self.voice.iter_mut().enumerate() {
                let p = voice.mutable_parameters();
                p.brightness = parameters.timbre;
                p.sustain = false;
                p.gate =
                    (parameters.trigger == TriggerState::High) && (i == self.active_voice as usize);
                if voice_patch_changed[i] {
                    voice.mutable_lfo().step(out.len() as f32);
                    voice.set_modulations(voice.lfo().pitch_mod(), voice.lfo().amp_mod());
                } else {
                    voice.set_modulations(active_voice_pitch_mod, active_voice_amp_mod);
                }
            }
        }

        // TODO: change hard-coded 2 voice rendering to generic rendering

        self.temp_buffer_2.fill(0.0);

        #[allow(clippy::useless_asref)]
        let mut buffers = [
            self.temp_buffer_1.as_mut(),
            self.temp_buffer_2.as_mut(),
            self.temp_buffer_3.as_mut(),
            self.temp_buffer_4.as_mut(),
        ];

        self.voice[0].render(&mut buffers);

        self.acc_buffer.copy_from_slice(self.temp_buffer_2);
        self.temp_buffer_2.fill(0.0);

        #[allow(clippy::useless_asref)]
        let mut buffers = [
            self.temp_buffer_1.as_mut(),
            self.temp_buffer_2.as_mut(),
            self.temp_buffer_3.as_mut(),
            self.temp_buffer_4.as_mut(),
        ];

        self.voice[1].render(&mut buffers);

        for (i, (out_sample, aux_sample)) in out.iter_mut().zip(aux.iter_mut()).enumerate() {
            *out_sample = soft_clip((self.temp_buffer_2[i] + self.acc_buffer[i]) * 0.25);
            *aux_sample = *out_sample;
        }
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
    pub fn set_modulations(&mut self, pitch_mod: f32, amp_mod: f32) {
        self.parameters.pitch_mod = pitch_mod;
        self.parameters.amp_mod = amp_mod;
    }
}
