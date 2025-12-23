#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod chords;
pub mod downsampler;
pub mod drums;
pub mod engine;
pub mod engine2;
pub mod envelope;
pub mod fm;
pub mod fx;
pub mod noise;
pub mod oscillator;
pub mod physical_modelling;
pub mod resources;
pub mod speech;
pub mod utils;
pub mod voice;

/// Sample rate context for DSP calculations.
#[derive(Debug, Clone, Copy)]
pub struct SampleRate {
    /// Sample rate in Hz
    pub sample_rate_hz: f32,
    /// Reciprocal of sample rate (1.0 / sample_rate_hz) for fast multiplication
    pub inv_sr: f32,
    /// Normalized frequency of A0 (55 Hz) for MIDI note conversion
    pub a0_normalized: f32,
}

impl SampleRate {
    /// Create a new sample rate context.
    pub fn new(sample_rate_hz: f32) -> Self {
        let inv_sr = 1.0 / sample_rate_hz;
        let a0_normalized = 55.0 * inv_sr;
        Self {
            sample_rate_hz,
            inv_sr,
            a0_normalized,
        }
    }
}
