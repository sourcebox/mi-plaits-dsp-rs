//! Fast 16-bit pseudo random number generator.

// Based on MIT-licensed code (c) 2012 by Olivier Gillet (ol.gillet@gmail.com)

use core::sync::atomic::{AtomicU32, Ordering};

static RNG_STATE: AtomicU32 = AtomicU32::new(0x21);

#[inline]
fn state() -> u32 {
    RNG_STATE.load(Ordering::Relaxed)
}

#[inline]
pub fn seed(seed: u32) {
    RNG_STATE.store(seed, Ordering::Relaxed);
}

#[inline]
pub fn get_word() -> u32 {
    RNG_STATE.store(
        RNG_STATE
            .load(Ordering::Relaxed)
            .wrapping_mul(1664525)
            .wrapping_add(1013904223),
        Ordering::Relaxed,
    );
    state()
}

#[inline]
pub fn get_sample() -> i16 {
    (get_word() >> 16) as i16
}

#[inline]
pub fn get_float() -> f32 {
    get_word() as f32 / 4294967296.0
}
