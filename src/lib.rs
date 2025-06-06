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

/// Audio sample rate in Hz.
pub const SAMPLE_RATE: f32 = 48000.0;

/// Normalized frequency of note A0.
pub const A0: f32 = (440.0 / 8.0) / SAMPLE_RATE;
