//! Top-level module for all DSP related code specific to the device.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

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
pub mod voice;

use core::alloc::{GlobalAlloc, Layout};

/// Audio sample rate in Hz.
pub const SAMPLE_RATE: f32 = 48000.0;

/// Normalized frequency of note A0.
pub const A0: f32 = (440.0 / 8.0) / SAMPLE_RATE;

/// Allocate a zeroed buffer of f32s with a given number of elements.
pub fn allocate_buffer<A: GlobalAlloc>(
    buffer_allocator: &A,
    buffer_length: usize,
) -> Result<&'static mut [f32], AllocError> {
    let size = buffer_length * core::mem::size_of::<f32>();
    let buffer = unsafe {
        buffer_allocator.alloc_zeroed(Layout::from_size_align(size, 8).unwrap()) as *mut f32
    };

    if buffer.is_null() {
        return Err(AllocError);
    }

    let buffer: &mut [f32] = unsafe { core::slice::from_raw_parts_mut(buffer, buffer_length) };

    Ok(buffer)
}

pub fn allocate<T, A: GlobalAlloc>(allocator: &A) -> Result<&'static mut T, AllocError> {
    let size = core::mem::size_of::<T>();
    let block =
        unsafe { allocator.alloc_zeroed(Layout::from_size_align(size, 8).unwrap()) as *mut T };

    if block.is_null() {
        return Err(AllocError);
    }

    let block = unsafe { block.as_mut() };

    block.ok_or(AllocError)
}

#[derive(Debug)]
pub struct AllocError;
