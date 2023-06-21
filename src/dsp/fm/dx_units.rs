//! Various "magic" conversion functions for DX7 patch data.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

#![allow(clippy::excessive_precision)]

#[allow(unused_imports)]
use num_traits::float::Float;

use super::patch::{KeyboardScaling, Operator};
use crate::stmlib::dsp::interpolate;
use crate::stmlib::dsp::units::semitones_to_ratio_safe;

// Computes 2^x by using a polynomial approximation of 2^frac(x) and directly
// incrementing the exponent of the IEEE 754 representation of the result
// by int(x). Depending on the use case, the order of the polynomial
// approximation can be chosen.
#[inline]
pub fn pow_2_fast(mut x: f32, order: i32) -> f32 {
    #[repr(C)]
    union Result {
        f: f32,
        w: i32,
    }

    let mut r = Result { f: 0.0 };

    if order == 1 {
        r.w = ((1 << 23) as f32 * (127.0 + x)) as i32;

        return unsafe { r.f };
    }

    let mut x_integral = x as i32;

    if x < 0.0 {
        x_integral -= 1;
    }

    x -= x_integral as f32;

    if order == 1 {
        r.f = 1.0 + x;
    } else if order == 2 {
        r.f = 1.0 + x * (0.6565 + x * 0.3435);
    } else if order == 3 {
        r.f = 1.0 + x * (0.6958 + x * (0.2251 + x * 0.0791));
    }

    unsafe {
        r.w += x_integral << 23;
    }

    unsafe { r.f }
}

// Convert an operator (envelope) level from 0-99 to the complement of the
// "TL" value.
//   0 =   0  (TL = 127)
//  20 =  48  (TL =  79)
//  50 =  78  (TL =  49)
//  99 = 127  (TL =   0)
#[inline]
pub fn operator_level(level: u8) -> u8 {
    let mut tlc = level as u32;

    if level < 20 {
        tlc = if tlc < 15 {
            (tlc * (36 - tlc)) >> 3
        } else {
            27 + tlc
        };
    } else {
        tlc += 28;
    }

    tlc as u8
}

// Convert an envelope level from 0-99 to an octave shift.
//  0 = -4 octave
// 18 = -1 octave
// 50 =  0
// 82 = +1 octave
// 99 = +4 octave
#[inline]
pub fn pitch_envelope_level(level: u8) -> f32 {
    let l = (level as f32 - 50.0) / 32.0;
    let tail = f32::max(f32::abs(l + 0.02) - 1.0, 0.0);

    l * (1.0 + tail * tail * 5.3056)
}

// Convert an operator envelope rate from 0-99 to a frequency.
#[inline]
pub fn operator_envelope_increment(rate: i32) -> f32 {
    let rate_scaled = (rate * 41) >> 6;
    let mantissa = 4 + (rate_scaled & 3);
    let exponent = 2 + (rate_scaled >> 2);

    (mantissa << exponent) as f32 / (1 << 24) as f32
}

// Convert a pitch envelope rate from 0-99 to a frequency.
#[inline]
pub fn pitch_envelope_increment(rate: u8) -> f32 {
    let r = rate as f32 * 0.01;

    (1.0 + 192.0 * r * (r * r * r * r + 0.3333)) / (21.3 * 44100.0)
}

const MIN_LFO_FREQUENCY: f32 = 0.005865;

// Convert an LFO rate from 0-99 to a frequency.
#[inline]
pub fn lfo_frequency(rate: u8) -> f32 {
    let mut rate_scaled = if rate == 0 {
        1
    } else {
        (rate as u32 * 165) >> 6
    };
    rate_scaled *= if rate_scaled < 160 {
        11
    } else {
        11 + ((rate_scaled - 160) >> 4)
    };

    rate_scaled as f32 * MIN_LFO_FREQUENCY
}

// Convert an LFO delay from 0-99 to the two increments.
#[inline]
pub fn lfo_delay(delay: u8, increments: &mut [f32; 2]) {
    if delay == 0 {
        increments[0] = 100000.0;
        increments[1] = increments[0];
    } else {
        let mut d = (99 - delay) as i32;
        d = (16 + (d & 15)) << (1 + (d >> 4));
        increments[0] = d as f32 * MIN_LFO_FREQUENCY;
        increments[1] = (i32::max(0x80, d & 0xFF80)) as f32 * MIN_LFO_FREQUENCY;
    }
}

// Pre-process the velocity to easily compute the velocity scaling.
#[inline]
pub fn normalize_velocity(velocity: f32) -> f32 {
    // float cube_root = stmlib::Sqrt(
    //     0.7f * stmlib::Sqrt(velocity) + 0.3f * velocity);
    let cube_root = interpolate(&LUT_CUBE_ROOT, velocity, 16.0);

    16.0 * (cube_root - 0.918)
}

// MIDI note to envelope increment ratio.
#[inline]
pub fn rate_scaling(note: f32, rate_scaling: u8) -> f32 {
    pow_2_fast(rate_scaling as f32 * (note * 0.33333 - 7.0) * 0.03125, 1)
}

// Operator amplitude modulation sensitivity (0-3).
#[inline]
pub fn amp_mod_sensitivity(amp_mod_sensitivity: u8) -> f32 {
    LUT_AMP_MOD_SENSITIVITY[amp_mod_sensitivity as usize]
}

// Pitch modulation sensitivity (0-7).
#[inline]
pub fn pitch_mod_sensitivity(pitch_mod_sensitivity: u8) -> f32 {
    LUT_PITCH_MOD_SENSITIVITY[pitch_mod_sensitivity as usize]
}

// Keyboard tracking to TL adjustment.
#[inline]
pub fn keyboard_scaling(note: f32, ks: &KeyboardScaling) -> f32 {
    let x = note - ks.break_point as f32 - 15.0;
    let curve = if x > 0.0 {
        ks.right_curve
    } else {
        ks.left_curve
    };

    let mut t = f32::abs(x);

    if curve == 1 || curve == 2 {
        t = f32::min(t * 0.010467, 1.0);
        t = t * t * t;
        t *= 96.0;
    }
    if curve < 2 {
        t = -t;
    }

    let depth = (if x > 0.0 {
        ks.right_depth
    } else {
        ks.left_depth
    }) as f32;

    t * depth * 0.02677
}

#[inline]
pub fn frequency_ratio(op: &Operator) -> f32 {
    let detune = if op.mode == 0 && op.fine != 0 {
        1.0 + 0.01 * op.fine as f32
    } else {
        1.0
    };

    let mut base = if op.mode == 0 {
        LUT_COARSE[op.coarse as usize]
    } else {
        ((op.coarse & 3) as i32 * 100 + op.fine as i32) as f32 * 0.39864
    };
    base += (op.detune as f32 - 7.0) * 0.015;

    semitones_to_ratio_safe(base) * detune
}

const LUT_COARSE: [f32; 32] = [
    -12.000000, 0.000000, 12.000000, 19.019550, 24.000000, 27.863137, 31.019550, 33.688259,
    36.000000, 38.039100, 39.863137, 41.513180, 43.019550, 44.405276, 45.688259, 46.882687,
    48.000000, 49.049554, 50.039100, 50.975130, 51.863137, 52.707809, 53.513180, 54.282743,
    55.019550, 55.726274, 56.405276, 57.058650, 57.688259, 58.295772, 58.882687, 59.450356,
];

const LUT_AMP_MOD_SENSITIVITY: [f32; 4] = [0.0, 0.2588, 0.4274, 1.0];

const LUT_PITCH_MOD_SENSITIVITY: [f32; 8] = [
    0.0, 0.0781250, 0.1562500, 0.2578125, 0.4296875, 0.7187500, 1.1953125, 2.0,
];

const LUT_CUBE_ROOT: [f32; 17] = [
    0.0,
    0.39685062976,
    0.50000000000,
    0.57235744065,
    0.62996081605,
    0.67860466725,
    0.72112502092,
    0.75914745216,
    0.79370070937,
    0.82548197054,
    0.85498810729,
    0.88258719406,
    0.90856038354,
    0.93312785379,
    0.95646563396,
    0.97871693135,
    1.0,
];
