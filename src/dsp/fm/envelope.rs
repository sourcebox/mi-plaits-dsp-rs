//! Multi-segment envelope generator.
//!
//! The classic constant-time design (as found in many other MI products) might
//! cause differences in behavior from the original DX-series envelopes, in
//! particular when jumping to the last segment before having reached the sustain
//! phase.
//!
//! The unusual RenderAtSample() method allows the evaluation of the envelope at
//! an arbitrary point in time, used in Plaits' "envelope scrubbing" feature.
//!
//! A couple of quirks from the DX-series' operator envelopes are implemented,
//! namely:
//! - vaguely logarithmic shape for ascending segments.
//! - direct jump above a threshold for ascending segments.
//! - specific logic and rates for plateaus.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use super::dx_units::{
    operator_envelope_increment, operator_level, pitch_envelope_increment, pitch_envelope_level,
};

pub const PREVIOUS_LEVEL: f32 = -100.0;

#[derive(Debug)]
pub struct Envelope<const NUM_STAGES: usize = 4, const RESHAPE_ASCENDING_SEGMENTS: bool = false> {
    pub stage: usize,
    pub phase: f32,
    pub start: f32,

    increment: [f32; NUM_STAGES],
    level: [f32; NUM_STAGES],
    scale: f32,
}

impl<const NUM_STAGES: usize, const RESHAPE_ASCENDING_SEGMENTS: bool>
    Envelope<NUM_STAGES, RESHAPE_ASCENDING_SEGMENTS>
{
    pub fn new() -> Self {
        Self {
            stage: 0,
            phase: 0.0,
            start: 0.0,

            increment: [0.0; NUM_STAGES],
            level: [0.0; NUM_STAGES],
            scale: 0.0,
        }
    }

    #[inline]
    pub fn init(&mut self, scale: f32) {
        self.scale = scale;
        self.stage = NUM_STAGES - 1;
        self.phase = 1.0;
        self.start = 0.0;

        for i in 0..NUM_STAGES {
            self.increment[i] = 0.001;
            self.level[i] = 1.0 / (1 << i) as f32;
        }

        self.level[NUM_STAGES - 1] = 0.0;
    }

    // Directly copy the variables.
    pub fn set(&mut self, increment: &[f32; NUM_STAGES], level: &[f32; NUM_STAGES]) {
        self.increment.copy_from_slice(increment);
        self.level.copy_from_slice(level);
    }

    #[inline]
    pub fn render_at_sample(&self, mut t: f32, gate_duration: f32) -> f32 {
        if t > gate_duration {
            // Check how far we are into the release phase.
            let phase = (t - gate_duration) * self.increment[NUM_STAGES - 1];
            return if phase >= 1.0 {
                self.level[NUM_STAGES - 1]
            } else {
                self.value(
                    NUM_STAGES - 1,
                    phase,
                    self.render_at_sample(gate_duration, gate_duration),
                )
            };
        }

        let mut stage = 0;

        while stage < NUM_STAGES - 1 {
            let stage_duration = 1.0 / self.increment[stage];
            if t < stage_duration {
                break;
            }
            t -= stage_duration;

            stage += 1;
        }

        if stage == NUM_STAGES - 1 {
            t -= gate_duration;
            if t <= 0.0 {
                // TODO(pichenettes): this should always be true.
                return self.level[NUM_STAGES - 2];
            } else if t * self.increment[NUM_STAGES - 1] > 1.0 {
                return self.level[NUM_STAGES - 1];
            }
        }

        self.value(stage, t * self.increment[stage], PREVIOUS_LEVEL)
    }

    #[inline]
    pub fn render_default(&mut self, gate: bool) -> f32 {
        self.render(gate, 1.0, 1.0, 1.0)
    }

    #[inline]
    pub fn render(&mut self, gate: bool, rate: f32, ad_scale: f32, release_scale: f32) -> f32 {
        if gate {
            if self.stage == NUM_STAGES - 1 {
                self.start = self.current_value();
                self.stage = 0;
                self.phase = 0.0;
            }
        } else if self.stage != NUM_STAGES - 1 {
            self.start = self.current_value();
            self.stage = NUM_STAGES - 1;
            self.phase = 0.0;
        }

        self.phase += self.increment[self.stage]
            * rate
            * (if self.stage == NUM_STAGES - 1 {
                release_scale
            } else {
                ad_scale
            });
        if self.phase >= 1.0 {
            if self.stage >= NUM_STAGES - 2 {
                self.phase = 1.0;
            } else {
                self.phase = 0.0;
                self.stage += 1;
            }
            self.start = PREVIOUS_LEVEL;
        }

        self.current_value()
    }

    #[inline]
    pub fn current_value(&self) -> f32 {
        self.value(self.stage, self.phase, self.start)
    }

    #[inline]
    pub fn value(&self, stage: usize, mut phase: f32, start_level: f32) -> f32 {
        let mut from = if start_level == PREVIOUS_LEVEL {
            self.level[(stage + NUM_STAGES - 1) % NUM_STAGES]
        } else {
            start_level
        };

        let mut to = self.level[stage];

        if RESHAPE_ASCENDING_SEGMENTS && from < to {
            from = f32::max(6.7, from);
            to = f32::max(6.7, to);
            phase *= (2.5 - phase) * 0.666667;
        }

        phase * (to - from) + from
    }
}

impl<const NUM_STAGES: usize, const RESHAPE_ASCENDING_SEGMENTS: bool> Default
    for Envelope<NUM_STAGES, RESHAPE_ASCENDING_SEGMENTS>
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct OperatorEnvelope<const NUM_STAGES: usize = 4>(pub Envelope<NUM_STAGES, true>);

impl<const NUM_STAGES: usize> OperatorEnvelope<NUM_STAGES> {
    pub fn new() -> Self {
        Self(Envelope::<NUM_STAGES, true>::new())
    }

    pub fn set(&mut self, rate: &[u8; NUM_STAGES], level: [u8; NUM_STAGES], global_level: u8) {
        // Configure levels.
        for (i, level) in level.iter().enumerate().take(NUM_STAGES) {
            let mut level_scaled = operator_level(*level) as i32;
            level_scaled = (level_scaled & !1) + global_level as i32 - 133; // 125 ?
            self.0.level[i] = 0.125
                * (if level_scaled < 1 {
                    0.5
                } else {
                    level_scaled as f32
                });
        }

        // Configure increments.
        for i in 0..NUM_STAGES {
            let mut increment = operator_envelope_increment(rate[i]);
            let mut from = self.0.level[(i + NUM_STAGES - 1) % NUM_STAGES];
            let mut to = self.0.level[i];

            if from == to {
                // Quirk: for plateaux, the increment is scaled.
                increment *= 0.6;
                if i == 0 && level[i] != 0 {
                    // Quirk: the attack plateau is faster.
                    increment *= 20.0;
                }
            } else if from < to {
                from = f32::max(6.7, from);
                to = f32::max(6.7, to);
                if from == to {
                    // Quirk: because of the jump, the attack might disappear.
                    increment = 1.0;
                } else {
                    // Quirk: because of the weird shape, the rate is adjusted.
                    increment *= 7.2 / (to - from);
                }
            } else {
                increment *= 1.0 / (from - to);
            }

            self.0.increment[i] = increment * self.0.scale;
        }
    }
}

impl<const NUM_STAGES: usize> Default for OperatorEnvelope<NUM_STAGES> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct PitchEnvelope<const NUM_STAGES: usize = 4>(pub Envelope<NUM_STAGES, false>);

impl<const NUM_STAGES: usize> PitchEnvelope<NUM_STAGES> {
    pub fn new() -> Self {
        Self(Envelope::<NUM_STAGES, false>::new())
    }

    pub fn set(&mut self, rate: [u8; NUM_STAGES], level: [u8; NUM_STAGES]) {
        // Configure levels.
        for (i, level) in level.iter().enumerate().take(NUM_STAGES) {
            self.0.level[i] = pitch_envelope_level(*level);
        }

        // Configure increments.
        for (i, rate) in rate.iter().enumerate().take(NUM_STAGES) {
            let from = self.0.level[(i + NUM_STAGES - 1) % NUM_STAGES];
            let to = self.0.level[i];
            let mut increment = pitch_envelope_increment(*rate);

            if from != to {
                increment *= 1.0 / f32::abs(from - to);
            } else if i != NUM_STAGES - 1 {
                increment = 0.2;
            }

            self.0.increment[i] = increment * self.0.scale;
        }
    }
}

impl<const NUM_STAGES: usize> Default for PitchEnvelope<NUM_STAGES> {
    fn default() -> Self {
        Self::new()
    }
}
