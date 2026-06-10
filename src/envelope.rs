//! Envelope for the internal LPG.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::utils::{scaled_smoothing_coefficient, REFERENCE_SAMPLE_RATE};

#[derive(Debug, Clone)]
pub struct LpgEnvelope {
    vactrol_state: f32,
    gain: f32,
    frequency: f32,
    hf_bleed: f32,
    ramp_up: bool,

    // Sample rate dependent constants
    frequency_scale: f32,
    attack_coefficient: f32,
}

impl Default for LpgEnvelope {
    fn default() -> Self {
        Self {
            vactrol_state: 0.0,
            gain: 1.0,
            frequency: 0.5,
            hf_bleed: 0.0,
            ramp_up: false,
            frequency_scale: 1.0,
            attack_coefficient: 0.6,
        }
    }
}

impl LpgEnvelope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        // The LPG filter cutoff constants are normalized frequencies at the
        // reference sample rate; rescale them so the cutoff stays constant
        // in Hz. The vactrol state is updated once per block; with a fixed
        // block size the block rate scales with the sample rate, so the same
        // ratio rescales its attack coefficient.
        let rate_ratio = REFERENCE_SAMPLE_RATE / sample_rate_hz;
        self.frequency_scale = rate_ratio;
        self.attack_coefficient = scaled_smoothing_coefficient(0.6, rate_ratio);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.vactrol_state = 0.0;
        self.gain = 1.0;
        self.frequency = 0.5;
        self.hf_bleed = 0.0;
        self.ramp_up = false;
    }

    pub fn trigger(&mut self) {
        self.ramp_up = true;
    }

    #[inline]
    pub fn process_ping(&mut self, attack: f32, short_decay: f32, decay_tail: f32, hf: f32) {
        if self.ramp_up {
            self.vactrol_state += attack;
            if self.vactrol_state >= 1.0 {
                self.vactrol_state = 1.0;
                self.ramp_up = false;
            }
        }
        self.process_lp(
            if self.ramp_up {
                self.vactrol_state
            } else {
                0.0
            },
            short_decay,
            decay_tail,
            hf,
        );
    }

    #[inline]
    pub fn process_lp(&mut self, level: f32, short_decay: f32, decay_tail: f32, hf: f32) {
        let vactrol_input = level;
        let vactrol_error = vactrol_input - self.vactrol_state;
        let vactrol_state_2 = self.vactrol_state * self.vactrol_state;
        let vactrol_state_4 = vactrol_state_2 * vactrol_state_2;
        let tail = 1.0 - self.vactrol_state;
        let tail_2 = tail * tail;
        let vactrol_coefficient = if vactrol_error > 0.0 {
            self.attack_coefficient
        } else {
            short_decay + (1.0 - vactrol_state_4) * decay_tail
        };
        self.vactrol_state += vactrol_coefficient * vactrol_error;

        self.gain = self.vactrol_state;
        self.frequency =
            ((0.003 + 0.3 * vactrol_state_4 + hf * 0.04) * self.frequency_scale).min(0.45);
        self.hf_bleed = (tail_2 + (1.0 - tail_2) * hf) * hf * hf;
    }

    #[inline]
    pub fn gain(&self) -> f32 {
        self.gain
    }

    #[inline]
    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    #[inline]
    pub fn hf_bleed(&self) -> f32 {
        self.hf_bleed
    }
}

#[derive(Debug, Default, Clone)]
pub struct DecayEnvelope {
    value: f32,
}

impl DecayEnvelope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.value = 0.0;
    }

    pub fn trigger(&mut self) {
        self.value = 1.0;
    }

    #[inline]
    pub fn process(&mut self, decay: f32) {
        self.value *= 1.0 - decay;
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.value
    }
}
