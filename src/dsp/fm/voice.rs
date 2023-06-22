//! DX7 voice.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

#[allow(unused_imports)]
use num_traits::float::Float;

use super::algorithms::Algorithms;
use super::dx_units::{
    amp_mod_sensitivity, frequency_ratio, keyboard_scaling, normalize_velocity, operator_level,
    pow_2_fast, rate_scaling,
};
use super::envelope::{OperatorEnvelope, PitchEnvelope};
use super::operator::Operator;
use super::patch::Patch;
use crate::dsp::allocate_buffer;
use crate::stmlib::dsp::units::semitones_to_ratio_safe;

#[derive(Debug, Default)]
pub struct VoiceParameters {
    pub sustain: bool,
    pub gate: bool,
    pub note: f32,
    pub velocity: f32,
    pub brightness: f32,
    pub envelope_control: f32,
    pub pitch_mod: f32,
    pub amp_mod: f32,
}

impl VoiceParameters {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug)]
pub struct Voice<'a, const NUM_OPERATORS: usize, const NUM_ALGORITHMS: usize> {
    algorithms: Option<&'a Algorithms<NUM_OPERATORS, NUM_ALGORITHMS>>,
    sample_rate: f32,
    one_hz: f32,
    a0: f32,

    gate: bool,

    operator: [Operator; NUM_OPERATORS],
    operator_envelope: [OperatorEnvelope; NUM_OPERATORS],
    pitch_envelope: PitchEnvelope,

    normalized_velocity: f32,
    note: f32,

    ratios: [f32; NUM_OPERATORS],
    level_headroom: [f32; NUM_OPERATORS],
    level: [f32; NUM_OPERATORS],

    feedback_state: [f32; 2],

    patch: Option<&'a Patch>,

    dirty: bool,

    temp_buffer: &'a mut [f32],
}

impl<'a, const NUM_OPERATORS: usize, const NUM_ALGORITHMS: usize>
    Voice<'a, NUM_OPERATORS, NUM_ALGORITHMS>
{
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            algorithms: None,
            sample_rate: 0.0,
            one_hz: 0.0,
            a0: 0.0,

            gate: false,

            operator: core::array::from_fn(|_| Operator::new()),
            operator_envelope: core::array::from_fn(|_| OperatorEnvelope::new()),
            pitch_envelope: PitchEnvelope::new(),

            normalized_velocity: 0.0,
            note: 0.0,

            ratios: [0.0; NUM_OPERATORS],
            level_headroom: [0.0; NUM_OPERATORS],
            level: [0.0; NUM_OPERATORS],

            feedback_state: [0.0; 2],

            patch: None,

            dirty: false,

            temp_buffer: allocate_buffer(buffer_allocator, block_size).unwrap(),
        }
    }

    #[inline]
    pub fn init(
        &mut self,
        algorithms: &'a Algorithms<NUM_OPERATORS, NUM_ALGORITHMS>,
        sample_rate: f32,
    ) {
        self.algorithms = Some(algorithms);

        self.sample_rate = sample_rate;
        self.one_hz = 1.0 / sample_rate;
        self.a0 = 55.0 / sample_rate;

        let native_sr = 44100.0; // Legacy sample rate.
        let envelope_scale = native_sr * self.one_hz;

        for (operator, operator_envelope) in self
            .operator
            .iter_mut()
            .zip(self.operator_envelope.iter_mut())
        {
            operator.reset();
            operator_envelope.0.init(envelope_scale);
        }

        self.pitch_envelope.0.init(envelope_scale);

        self.feedback_state[0] = 0.0;
        self.feedback_state[1] = 0.0;

        self.patch = None;
        self.gate = false;
        self.note = 48.0;
        self.normalized_velocity = 10.0;

        self.dirty = true;
    }

    #[inline]
    pub fn set_patch(&mut self, patch: Option<&'a Patch>) {
        self.patch = patch;
        self.dirty = true;
    }

    // Pre-compute everything that can be pre-computed once a patch is loaded:
    // - envelope constants
    // - frequency ratios
    #[inline]
    pub fn setup(&mut self) -> bool {
        if !self.dirty {
            return false;
        }

        if let Some(patch) = self.patch {
            self.pitch_envelope
                .set(&patch.pitch_envelope.rate, &patch.pitch_envelope.level);

            for i in 0..NUM_OPERATORS {
                let op = &patch.op[i];

                let level = operator_level(op.level);
                self.operator_envelope[i].set(&op.envelope.rate, &op.envelope.level, level);

                // The level increase caused by keyboard scaling plus velocity
                // scaling should not exceed this number - otherwise it would be
                // equivalent to have an operator with a level above 99.
                self.level_headroom[i] = (127 - level) as f32;

                // Pre-compute frequency ratios. Encode the base frequency
                // (1Hz or the root note) as the sign of the ratio.
                let sign = if op.mode == 0 { 1.0 } else { -1.0 };
                self.ratios[i] = sign * frequency_ratio(op);
            }

            self.dirty = false;
        }

        true
    }

    #[inline]
    pub fn op_level(&self, i: u32) -> f32 {
        self.level[i as usize]
    }

    #[inline]
    pub fn render(&mut self, parameters: &VoiceParameters, buffers: &mut [&mut [f32]; 4]) {
        if self.setup() {
            // This prevents a CPU overrun, since there is not enough CPU to perform
            // both a patch setup and a full render in the time alloted for
            // a render. As a drawback, this causes a 0.5ms blank before a new
            // patch starts playing. But this is a clean blank, as opposed to a
            // glitchy overrun.
            return;
        }

        let envelope_rate = buffers[0].len() as f32;
        let ad_scale = pow_2_fast((0.5 - parameters.envelope_control) * 8.0, 1);
        let r_scale = pow_2_fast(-f32::abs(parameters.envelope_control - 0.3) * 8.0, 1);
        let gate_duration = 1.5 * self.sample_rate;
        let envelope_sample = gate_duration * parameters.envelope_control;

        // Apply LFO and pitch envelope modulations.
        let pitch_envelope = if parameters.sustain {
            self.pitch_envelope
                .0
                .render_at_sample(envelope_sample, gate_duration)
        } else {
            self.pitch_envelope
                .0
                .render(parameters.gate, envelope_rate, ad_scale, r_scale)
        };
        let pitch_mod = pitch_envelope + parameters.pitch_mod;
        let f0 = self.a0 * 0.25 * semitones_to_ratio_safe(parameters.note - 9.0 + pitch_mod * 12.0);

        // Sample the note and velocity (used for scaling) only when a trigger
        // is received, or constantly when we are in free-running mode.
        let note_on = parameters.gate && !self.gate;
        self.gate = parameters.gate;

        if note_on || parameters.sustain {
            self.normalized_velocity = normalize_velocity(parameters.velocity);
            self.note = parameters.note;
        }

        if let Some(patch) = self.patch {
            // Reset operator phase if a note on is detected & if the patch requires it.
            if note_on && patch.reset_phase != 0 {
                for i in 0..NUM_OPERATORS {
                    self.operator[i].phase = 0;
                }
            }

            // Compute frequencies and amplitudes.
            let mut f = [0.0; NUM_OPERATORS];
            let mut a = [0.0; NUM_OPERATORS];

            for i in 0..NUM_OPERATORS {
                let op = &patch.op[i];

                f[i] = self.ratios[i]
                    * (if self.ratios[i] < 0.0 {
                        -self.one_hz
                    } else {
                        f0
                    });

                let rate_scaling = rate_scaling(self.note, op.rate_scaling);
                let mut level = if parameters.sustain {
                    self.operator_envelope[i]
                        .0
                        .render_at_sample(envelope_sample, gate_duration)
                } else {
                    self.operator_envelope[i].0.render(
                        parameters.gate,
                        envelope_rate * rate_scaling,
                        ad_scale,
                        r_scale,
                    )
                };
                let kb_scaling = keyboard_scaling(self.note, &op.keyboard_scaling);
                let velocity_scaling = self.normalized_velocity * op.velocity_sensitivity as f32;
                let brightness = if self
                    .algorithms
                    .unwrap()
                    .is_modulator(patch.algorithm as u32, i as u32)
                {
                    (parameters.brightness - 0.5) * 32.0
                } else {
                    0.0
                };

                level += 0.125
                    * f32::min(
                        kb_scaling + velocity_scaling + brightness,
                        self.level_headroom[i],
                    );

                self.level[i] = level;

                let sensitivity = amp_mod_sensitivity(op.amp_mod_sensitivity);
                let log_level_mod = sensitivity * parameters.amp_mod - 1.0;
                let level_mod = 1.0 - pow_2_fast(6.4 * log_level_mod, 2);
                a[i] = pow_2_fast(-14.0 + level * level_mod, 2);
            }

            let mut i = 0;

            while i < NUM_OPERATORS {
                let call = self
                    .algorithms
                    .unwrap()
                    .render_call(patch.algorithm as u32, i as u32);

                if let Some(render_fn) = call.render_fn {
                    render_fn(
                        &mut self.operator[i..],
                        &f[i..],
                        &a[i..],
                        &mut self.feedback_state,
                        patch.feedback as i32,
                        buffers[call.input_index as usize],
                        self.temp_buffer,
                    );

                    buffers[call.output_index as usize].copy_from_slice(self.temp_buffer);
                }

                i += call.n as usize;
            }
        }
    }
}
