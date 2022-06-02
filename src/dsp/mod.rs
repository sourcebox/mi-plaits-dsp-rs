//! Utility DSP routines.

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

pub const SAMPLE_RATE: f32 = 48000.0;

// There is no proper PLL for I2S, only a divider on the system clock to derive
// the bit clock.
// The division ratio is set to 47 (23 EVEN, 1 ODD) by the ST libraries.
//
// Bit clock = 72000000 / 47 = 1531.91 kHz
// Frame clock = Bit clock / 32 = 47872.34 Hz
//
// That's only 4.6 cts of error, but we care!

use core::alloc::{GlobalAlloc, Layout};

pub const CORRECTED_SAMPLE_RATE: f32 = 47872.34;
pub const A0: f32 = (440.0 / 8.0) / CORRECTED_SAMPLE_RATE;

pub const MAX_BLOCK_SIZE: usize = 24;
pub const BLOCK_SIZE: usize = 12;

/// Allocate a zeroed buffer of f32s with a given number of elements
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
