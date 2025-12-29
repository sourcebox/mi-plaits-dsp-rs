//! Naive snare drum model (two modulated oscillators + filtered noise).
//!
//! Uses a few magic numbers taken from the 909 schematics:
//! - Ratio between the two modes of the drum set to 1.47.
//! - Funky coupling between the two modes.
//! - Noise coloration filters and envelope shapes for the snare.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use crate::utils::filter::{FilterMode, FrequencyApproximation, OnePole, Svf};
use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::random;
use crate::utils::sqrt;
use crate::utils::units::semitones_to_ratio;

#[derive(Debug, Default, Clone)]
pub struct SyntheticSnareDrum {
    sample_rate_hz: f32,
    phase: [f32; 2],
    drum_amplitude: f32,
    snare_amplitude: f32,
    fm: f32,
    sustain_gain: f32,
    hold_counter: i32,

    drum_lp: OnePole,
    snare_hp: OnePole,
    snare_lp: Svf,
}

impl SyntheticSnareDrum {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        self.sample_rate_hz = sample_rate_hz;
        self.phase = [0.0; 2];
        self.drum_amplitude = 0.0;
        self.snare_amplitude = 0.0;
        self.fm = 0.0;
        self.hold_counter = 0;
        self.sustain_gain = 0.0;

        self.drum_lp.init();
        self.snare_hp.init();
        self.snare_lp.init();
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        sustain: bool,
        trigger: bool,
        accent: f32,
        f0: f32,
        mut fm_amount: f32,
        decay: f32,
        mut snappy: f32,
        out: &mut [f32],
    ) {
        let decay_xt = decay * (1.0 + decay * (decay - 1.0));
        fm_amount *= fm_amount;
        let drum_decay = 1.0
            - 1.0 / (0.015 * self.sample_rate_hz)
                * semitones_to_ratio(-decay_xt * 72.0 - fm_amount * 12.0 + snappy * 7.0);
        let snare_decay =
            1.0 - 1.0 / (0.01 * self.sample_rate_hz) * semitones_to_ratio(-decay * 60.0 - snappy * 7.0);
        let fm_decay = 1.0 - 1.0 / (0.007 * self.sample_rate_hz);

        snappy = snappy * 1.1 - 0.05;
        snappy = snappy.clamp(0.0, 1.0);

        let drum_level = sqrt(1.0 - snappy);
        let snare_level = sqrt(snappy);

        let snare_f_min = f32::min(10.0 * f0, 0.5);
        let snare_f_max = f32::min(35.0 * f0, 0.5);

        self.snare_hp
            .set_f(snare_f_min, FrequencyApproximation::Fast);
        self.snare_lp.set_f_q(
            snare_f_max,
            0.5 + 2.0 * snappy,
            FrequencyApproximation::Fast,
        );
        self.drum_lp.set_f(3.0 * f0, FrequencyApproximation::Fast);

        if trigger {
            self.snare_amplitude = 0.3 + 0.7 * accent;
            self.drum_amplitude = self.snare_amplitude;
            self.fm = 1.0;
            self.phase[0] = 0.0;
            self.phase[1] = 0.0;
            self.hold_counter = ((0.04 + decay * 0.03) * self.sample_rate_hz) as i32;
        }

        let mut sustain_gain =
            ParameterInterpolator::new(&mut self.sustain_gain, accent * decay, out.len());

        let size = out.len();

        for out_sample in out.iter_mut() {
            if sustain {
                self.snare_amplitude = sustain_gain.next();
                self.drum_amplitude = self.snare_amplitude;
                self.fm = 0.0;
            } else {
                // Compute all D envelopes.
                // The envelope for the drum has a very long tail.
                // The envelope for the snare has a "hold" stage which lasts
                // between 40 and 70 ms
                self.drum_amplitude *= if self.drum_amplitude > 0.03 || !(size & 1) != 0 {
                    drum_decay
                } else {
                    1.0
                };
                if self.hold_counter != 0 {
                    self.hold_counter -= 1;
                } else {
                    self.snare_amplitude *= snare_decay;
                }
                self.fm *= fm_decay;
            }

            // The 909 circuit has a funny kind of oscillator coupling - the
            // signal leaving Q40's collector and resetting all oscillators
            // allow some intermodulation.
            let mut reset_noise = 0.0;
            let mut reset_noise_amount = (0.125 - f0) * 8.0;
            reset_noise_amount = reset_noise_amount.clamp(0.0, 1.0);
            reset_noise_amount *= reset_noise_amount;
            reset_noise_amount *= fm_amount;
            reset_noise += if self.phase[0] > 0.5 { -1.0 } else { 1.0 };
            reset_noise += if self.phase[1] > 0.5 { -1.0 } else { 1.0 };
            reset_noise *= reset_noise_amount * 0.025;

            let f = f0 * (1.0 + fm_amount * (4.0 * self.fm));
            self.phase[0] += f;
            self.phase[1] += f * 1.47;

            if reset_noise_amount > 0.1 {
                if self.phase[0] >= 1.0 + reset_noise {
                    self.phase[0] = 1.0 - self.phase[0];
                }
                if self.phase[1] >= 1.0 + reset_noise {
                    self.phase[1] = 1.0 - self.phase[1];
                }
            } else {
                if self.phase[0] >= 1.0 {
                    self.phase[0] -= 1.0;
                }
                if self.phase[1] >= 1.0 {
                    self.phase[1] -= 1.0;
                }
            }

            let mut drum = -0.1;
            drum += Self::distorted_sine(self.phase[0]) * 0.60;
            drum += Self::distorted_sine(self.phase[1]) * 0.25;
            drum *= self.drum_amplitude * drum_level;
            drum = self.drum_lp.process(drum, FilterMode::LowPass);

            let noise = random::get_float();
            let mut snare = self.snare_lp.process(noise, FilterMode::LowPass);
            snare = self.snare_hp.process(snare, FilterMode::HighPass);
            snare = (snare + 0.1) * (self.snare_amplitude + self.fm) * snare_level;

            *out_sample = snare + drum; // It's a snare, it's a drum, it's a snare drum.
        }
    }

    #[inline]
    fn distorted_sine(phase: f32) -> f32 {
        let triangle = (if phase < 0.5 { phase } else { 1.0 - phase }) * 4.0 - 1.3;

        2.0 * triangle / (1.0 + triangle.abs())
    }
}
