//! Fast reciprocal of square-root routines.

// Based on MIT-licensed code (c) 2014 by Olivier Gillet (ol.gillet@gmail.com)

#[inline]
pub fn fast_rsqrt_carmack(x: f32) -> f32 {
    const THREEHALFS: f32 = 1.5;
    let mut y = x;
    let mut i = y as u32;
    i = 0x5f3759df - (i >> 1);
    y = i as f32;
    let x2 = x * 0.5;
    y = y * (THREEHALFS - (x2 * y * y));

    y
}
