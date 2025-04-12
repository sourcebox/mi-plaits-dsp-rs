//! Top-level module for the engines.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

pub mod additive_engine;
pub mod bass_drum_engine;
pub mod chord_engine;
pub mod fm_engine;
pub mod grain_engine;
pub mod hihat_engine;
pub mod modal_engine;
pub mod noise_engine;
pub mod particle_engine;
pub mod snare_drum_engine;
pub mod speech_engine;
pub mod string_engine;
pub mod swarm_engine;
pub mod virtual_analog_engine;
pub mod waveshaping_engine;
pub mod wavetable_engine;

use crate::utils::units::semitones_to_ratio;
use crate::A0;

pub trait Engine {
    fn init(&mut self);

    fn reset(&mut self) {}

    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        already_enveloped: &mut bool,
    );
}

#[derive(Debug, Default)]
pub struct EngineParameters {
    /// Trigger signal state
    pub trigger: TriggerState,

    /// Pitch in semitones
    /// Range: -119.0 - 120.0
    pub note: f32,

    /// Sweeps the spectral content from dark/sparse to bright/dense.
    /// Range: 0.0 - 1.0
    pub timbre: f32,

    /// Lateral timbral variations.
    /// Range: 0.0 - 1.0
    pub morph: f32,

    /// Frequency spread or the balance between the various constituents of the tone.
    /// Range: 0.0 - 1.0
    pub harmonics: f32,

    /// Level setting
    /// Range: 0.0 - 1.0
    pub accent: f32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TriggerState {
    Low = 0,
    RisingEdge = 1,
    Unpatched = 2,
    High = 4,
}

impl Default for TriggerState {
    fn default() -> Self {
        Self::Low
    }
}

#[inline]
pub fn note_to_frequency(mut midi_note: f32) -> f32 {
    midi_note -= 9.0;
    midi_note = midi_note.clamp(-128.0, 127.0);

    A0 * 0.25 * semitones_to_ratio(midi_note)
}
