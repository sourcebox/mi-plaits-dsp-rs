//! Random impulse train processed by a resonant filter.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::utils::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::utils::random;
use crate::utils::sqrt;
use crate::utils::units::semitones_to_ratio;

#[derive(Debug, Default)]
pub struct Particle {
    pre_gain: f32,
    filter: Svf,
}

impl Particle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.pre_gain = 0.0;
        self.filter.init();
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        sync: bool,
        density: f32,
        gain: f32,
        frequency: f32,
        spread: f32,
        q: f32,
        out: &mut [f32],
        aux: &mut [f32],
    ) {
        let mut u = random::get_float();
        if sync {
            u = density;
        }
        let mut can_radomize_frequency = true;

        for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter_mut()) {
            let mut s = 0.0;
            if u <= density {
                s = u * gain;
                if can_radomize_frequency {
                    let u = 2.0 * random::get_float() - 1.0;
                    let f = f32::min(semitones_to_ratio(spread * u) * frequency, 0.25);
                    self.pre_gain = 0.5 / sqrt(q * f * sqrt(density));
                    self.filter.set_f_q(f, q, FrequencyApproximation::Dirty);
                    // Keep the cutoff constant for this whole block.
                    can_radomize_frequency = false;
                }
            }
            *aux_sample += s;
            *out_sample += self.filter.process(self.pre_gain * s, FilterMode::BandPass);
            u = random::get_float();
        }
    }
}
