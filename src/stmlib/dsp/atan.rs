//! Fast arc-tangent routines.

// Based on MIT-licensed code (c) 2014 by Olivier Gillet (ol.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::FloatCore;

#[inline]
pub fn fast_atan2(y: f32, x: f32) -> u16 {
    const SIGN_MASK: u32 = 0x80000000;
    const B: f32 = 0.596227;
    let ux_s = SIGN_MASK & (x as u32);
    let uy_s = SIGN_MASK & (y as u32);
    let offset = (((!ux_s & uy_s) >> 29) | (ux_s >> 30)) << 14;
    let bxy_a = (B * x * y).abs();
    let num = bxy_a + y * y;
    let atan_1q = num / (x * x + bxy_a + num);
    let uatan_2q = (ux_s ^ uy_s) | (atan_1q as u32);
    (uatan_2q * 16384 + offset) as u16
}
