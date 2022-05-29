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

#[inline]
fn fast_rsqrt_accurate(fp0: f32) -> f32 {
    let _min = 1.0e-38;
    let _1p5 = 1.5;

    let q = fp0 as u32;
    let mut fp2 = (0x5F3997BB - ((q >> 1) & 0x3FFFFFFF)) as f32;
    let fp1 = _1p5 * fp0 - fp0;
    let mut fp3 = fp2 * fp2;

    if fp0 < _min {
        return if fp0 > 0.0 { fp2 } else { 1000.0 };
    }

    fp3 = _1p5 - fp1 * fp3;
    fp2 *= fp3;
    fp3 = fp2 * fp2;
    fp3 = _1p5 - fp1 * fp3;
    fp2 *= fp3;
    fp3 = fp2 * fp2;
    fp3 = _1p5 - fp1 * fp3;

    fp2 * fp3
}
