//! Top-level module for all DSP related code specific to the device.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

pub mod drums;
pub mod engine;
pub mod envelope;
pub mod fx;
pub mod noise;
pub mod oscillator;
pub mod physical_modelling;
pub mod resources;
pub mod speech;
pub mod voice;

use core::alloc::{GlobalAlloc, Layout};

/// Audio sample rate in Hz.
pub const SAMPLE_RATE: f32 = 48000.0;

/// Normalized frequency of note A0.
pub const A0: f32 = (440.0 / 8.0) / SAMPLE_RATE;

/// Allocate a zeroed buffer of f32s with a given number of elements.
pub fn allocate_buffer<T: GlobalAlloc>(
    buffer_allocator: &T,
    buffer_length: usize,
) -> &'static mut [f32] {
    let size = buffer_length * core::mem::size_of::<f32>();
    let buffer = unsafe {
        buffer_allocator.alloc_zeroed(Layout::from_size_align(size, 8).unwrap()) as *mut f32
    };
    let buffer: &mut [f32] = unsafe { core::slice::from_raw_parts_mut(buffer, buffer_length) };

    buffer
}
