//! 6-operator FM synth with 32 presets.
//!
//! Engine parameters:
//! - *HARMONICS:* preset selection.
//! - *TIMBRE:* modulator(s) level.
//! - *MORPH:* envelope and modula�on stretching/time-travel.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;
use core::cell::RefCell;

use spin::Once;

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

static ALGORITHMS: Once<Algorithms<6, 32>> = Once::new();

static mut PATCHES: Option<[Patch; NUM_PATCHES_PER_BANK]> = None;

#[derive(Debug)]
pub struct SixOpEngine<'a> {
    patch_index_quantizer: HysteresisQuantizer2,
    voice: [FmVoice<'a>; NUM_SIX_OP_VOICES],

    temp_buffer: &'a mut [f32],

    active_voice: i32,
    rendered_voice: i32,
}

impl SixOpEngine<'_> {
    pub fn new<A: GlobalAlloc>(buffer_allocator: &A, block_size: usize) -> Self {
        unsafe {
            let patches: [Patch; NUM_PATCHES_PER_BANK] = core::array::from_fn(|_| Patch::new());
            PATCHES = Some(patches);
        }
        Self {
            patch_index_quantizer: HysteresisQuantizer2::new(),
            voice: core::array::from_fn(|_| FmVoice::new(buffer_allocator, block_size)),
            temp_buffer: allocate_buffer(buffer_allocator, block_size).unwrap(),
            active_voice: 0,
            rendered_voice: 0,
        }
    }

    pub fn load_syx_bank(&mut self, bank: &[u8; 4096]) {
        let patches = unsafe { PATCHES.as_mut().unwrap() };

        for (i, patch) in patches.iter_mut().enumerate() {
            (*patch).unpack(&bank[i * SYX_SIZE..]);
        }

        for voice in self.voice.iter_mut() {
            voice.unload_patch();
        }
    }
}

impl Engine for SixOpEngine<'_> {
    fn init(&mut self) {
        self.patch_index_quantizer.init(32, 0.005, false);

        for voice in self.voice.iter_mut() {
            voice.init(
                ALGORITHMS.call_once(|| {
                    let mut algo = Algorithms::<6, 32>::new();
                    algo.init();
                    algo
                }),
                SAMPLE_RATE,
            );
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
            .process(parameters.harmonics * 1.02) as usize;

        if parameters.trigger == TriggerState::Unpatched {
            let t = parameters.morph;
            self.voice[0].mutable_lfo().scrub(2.0 * SAMPLE_RATE * t);

            let pitch_mod = self.voice[0].lfo().pitch_mod();
            let amp_mod = self.voice[0].lfo().amp_mod();

            for (i, voice) in self.voice.iter_mut().enumerate() {
                let patches = unsafe { PATCHES.as_ref().unwrap() };
                voice.load_patch(Some(&patches[patch_index]));
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
                self.voice[self.active_voice as usize].load_patch(Some(&patches[patch_index]));
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

        out.fill(0.0);

        for voice in self.voice.iter_mut() {
            self.temp_buffer.fill(0.0);

            voice.render(self.temp_buffer);

            for (out_sample, temp_sample) in out.iter_mut().zip(self.temp_buffer.iter()) {
                *out_sample = soft_clip(*out_sample + *temp_sample * 0.25);
            }
        }

        aux.copy_from_slice(out);
    }
}

#[derive(Debug)]
pub struct FmVoice<'a> {
    patch: Option<&'a Patch>,

    lfo: Lfo,
    voice: Voice<'a, 6, 32>,
    parameters: VoiceParameters,

    temp_buffer_1: &'a mut [f32],
    temp_buffer_2: &'a mut [f32],
    temp_buffer_3: &'a mut [f32],
}

impl<'a> FmVoice<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            patch: None,
            lfo: Lfo::new(),
            voice: Voice::<'a, 6, 32>::new(),
            parameters: VoiceParameters::new(),
            temp_buffer_1: allocate_buffer(buffer_allocator, block_size).unwrap(),
            temp_buffer_2: allocate_buffer(buffer_allocator, block_size).unwrap(),
            temp_buffer_3: allocate_buffer(buffer_allocator, block_size).unwrap(),
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
    pub fn render(&mut self, out: &mut [f32]) {
        if self.patch.is_none() {
            return;
        }

        let buffers = [
            RefCell::new(out),
            RefCell::new(self.temp_buffer_1),
            RefCell::new(self.temp_buffer_2),
            RefCell::new(self.temp_buffer_3),
        ];

        self.voice.render(&self.parameters, &buffers);
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
