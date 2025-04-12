//! Utility functions.
//!
//! This module contains ports of functions that were used in several Mutable Instruments
//! devices in common and were made for the STM32 platform.

pub mod atan;
pub mod cosine_oscillator;
pub mod delay_line;
pub mod filter;
pub mod hysteresis_quantizer;
pub mod limiter;
pub mod parameter_interpolator;
pub mod polyblep;
pub mod random;
pub mod rsqrt;
pub mod units;

#[allow(unused_imports)]
use num_traits::float::Float;

#[inline]
pub fn interpolate(table: &[f32], mut index: f32, size: f32) -> f32 {
    index = index.clamp(0.0, 1.0);
    index *= size;
    let index_integral = index as usize;
    let index_fractional = index - (index_integral as f32);
    let a = table[index_integral];
    let b = table[index_integral + 1];

    a + (b - a) * index_fractional
}

#[inline]
pub fn interpolate_hermite(table: &[f32], mut index: f32, size: f32) -> f32 {
    index = index.clamp(0.0, 1.0);
    index *= size;
    let index_integral = index as usize;
    let index_fractional = index - (index_integral as f32);
    let xm1 = table[index_integral - 1];
    let x0 = table[index_integral];
    let x1 = table[index_integral + 1];
    let x2 = table[index_integral + 2];
    let c = (x1 - xm1) * 0.5;
    let v = x0 - x1;
    let w = c + v;
    let a = w + v + (x2 - x0) * 0.5;
    let b_neg = w + a;
    let f = index_fractional;

    (((a * f) - b_neg) * f + c) * f + x0
}

#[inline]
pub fn interpolate_wrap(table: &[f32], mut index: f32, size: f32) -> f32 {
    index -= (index as i32) as f32;
    index = index.clamp(0.0, 1.0);
    index *= size;
    let index_integral = index as usize;
    let index_fractional = index - (index_integral as f32);
    let a = table[index_integral];
    let b = table[index_integral + 1];

    a + (b - a) * index_fractional
}

#[inline]
pub fn one_pole(out: &mut f32, in_: f32, coefficient: f32) {
    *out += (coefficient) * ((in_) - *out);
}

#[inline]
pub fn slope(out: &mut f32, in_: f32, positive: f32, negative: f32) {
    let error = in_ - *out;
    *out += if error > 0.0 {
        positive
    } else {
        negative * error
    };
}

#[inline]
pub fn slew(out: &mut f32, in_: f32, delta: f32) {
    let mut error = (in_) - *out;
    let d = delta;
    if error > d {
        error = d;
    } else if error < -d {
        error = -d;
    }
    *out += error;
}

#[inline]
pub fn crossfade(a: f32, b: f32, fade: f32) -> f32 {
    a + (b - a) * fade
}

#[inline]
pub fn soft_limit(x: f32) -> f32 {
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

#[inline]
pub fn soft_clip(x: f32) -> f32 {
    if x < -3.0 {
        -1.0
    } else if x > 3.0 {
        1.0
    } else {
        soft_limit(x)
    }
}

#[inline]
pub fn clip_16(x: i32) -> i32 {
    x.clamp(-32768, 32767)
}

#[inline]
pub fn sqrt(x: f32) -> f32 {
    f32::sqrt(x)
}
