//! Envelope for the internal LPG.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[derive(Debug, Default)]
pub struct LpgEnvelope {
    vactrol_state: f32,
    gain: f32,
    frequency: f32,
    hf_bleed: f32,
    ramp_up: bool,
}

impl LpgEnvelope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
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
            0.6
        } else {
            short_decay + (1.0 - vactrol_state_4) * decay_tail
        };
        self.vactrol_state += vactrol_coefficient * vactrol_error;

        self.gain = self.vactrol_state;
        self.frequency = 0.003 + 0.3 * vactrol_state_4 + hf * 0.04;
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

#[derive(Debug, Default)]
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
