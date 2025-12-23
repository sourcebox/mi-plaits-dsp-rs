//! Granular formant oscillator.
//!
//! Simulation of formants and filtered waveforms through the multiplication, addition and
//! synchronization of segments of sine waves.
//!
//! Engine parameters:
//! - *HARMONICS:* frequency ratio between formant 1 and 2.
//! - *TIMBRE:* formant frequency.
//! - *MORPH:* formant width and shape. This controls the shape of the window by which a sum
//!   of two synchronized sine oscillators is multiplied.
//!
//! *AUX* signal: simulation of filtered waveforms by windowed sine waves -
//! a recreation of Braids’ Z*** models. *HARMONICS* controls the filter type (peaking, LP, BP, HP),
//! with smooth variation from one response to another.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters};
use crate::oscillator::grainlet_oscillator::GrainletOscillator;
use crate::oscillator::z_oscillator::ZOscillator;
use crate::utils::filter::{FilterMode, FrequencyApproximation, OnePole};
use crate::utils::units::semitones_to_ratio;

#[derive(Debug, Default)]
pub struct GrainEngine {
    grainlet: [GrainletOscillator; 2],
    z_oscillator: ZOscillator,
    dc_blocker: [OnePole; 2],
}

impl GrainEngine {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Engine for GrainEngine {
    fn init(&mut self, _sample_rate_hz: f32) {
        self.grainlet[0].init();
        self.grainlet[1].init();
        self.z_oscillator.init();
        self.dc_blocker[0].init();
        self.dc_blocker[1].init();
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        let root = parameters.note;
        let f0 = note_to_frequency(root, parameters.a0_normalized);

        let f1 = note_to_frequency(24.0 + 84.0 * parameters.timbre, parameters.a0_normalized);
        let ratio = semitones_to_ratio(-24.0 + 48.0 * parameters.harmonics);
        let carrier_bleed = if parameters.harmonics < 0.5 {
            1.0 - 2.0 * parameters.harmonics
        } else {
            0.0
        };
        let carrier_bleed_fixed = carrier_bleed * (2.0 - carrier_bleed);
        let carrier_shape = 0.33 + (parameters.morph - 0.33) * f32::max(1.0 - f0 * 24.0, 0.0);

        self.grainlet[0].render(f0, f1, carrier_shape, carrier_bleed_fixed, out);
        self.grainlet[1].render(f0, f1 * ratio, carrier_shape, carrier_bleed_fixed, aux);
        self.dc_blocker[0].set_f(0.3 * f0, FrequencyApproximation::Dirty);

        for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter()) {
            *out_sample =
                self.dc_blocker[0].process(*out_sample + *aux_sample, FilterMode::HighPass);
        }

        let cutoff = note_to_frequency(root + 96.0 * parameters.timbre, parameters.a0_normalized);
        self.z_oscillator
            .render(f0, cutoff, parameters.morph, parameters.harmonics, aux);

        self.dc_blocker[1].set_f(0.3 * f0, FrequencyApproximation::Dirty);
        self.dc_blocker[1].process(aux[0], FilterMode::HighPass);
    }
}
