//! Fast reciprocal of square-root routines.

// Based on MIT-licensed code (c) 2014 by Olivier Gillet (ol.gillet@gmail.com)

/// Inverse square root approximation as implemented by John Carmack.
#[inline]
pub fn fast_rsqrt_carmack(x: f32) -> f32 {
    if x > 0.0 {
        const THREEHALFS: f32 = 1.5;

        let mut y = x;
        let mut i = y.to_bits();
        i = 0x5f3759df - (i >> 1);
        y = f32::from_bits(i);
        let x2 = x * 0.5;

        y * (THREEHALFS - (x2 * y * y))
    } else {
        f32::INFINITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_carmack() {
        for n in 0..100 {
            let x = n as f32;
            let result = fast_rsqrt_carmack(x);
            let expected = 1.0 / x.sqrt();
            let delta = ((result - expected) / expected).abs();

            if delta.is_nan() {
                assert!(result.is_infinite());
                assert!(expected.is_infinite());
            } else {
                assert!(
                    delta < 0.005,
                    "x= {x:.3}, result={result:.3}, expected={expected:.3}, delta={delta:.3}"
                );
            }
        }
    }
}
