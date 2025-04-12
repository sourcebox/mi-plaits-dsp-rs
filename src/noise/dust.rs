//! Randomly clocked samples.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::utils::random;

#[inline]
pub fn dust(frequency: f32) -> f32 {
    let inv_frequency = 1.0 / frequency;
    let u = random::get_float();

    if u < frequency {
        u * inv_frequency
    } else {
        0.0
    }
}
