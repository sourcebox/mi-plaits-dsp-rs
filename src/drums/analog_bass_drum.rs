//! 808 bass drum model, revisited.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use crate::oscillator::sine_oscillator::SineOscillator;
use crate::utils::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::utils::one_pole;
use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::units::semitones_to_ratio;

#[derive(Debug, Default, Clone)]
pub struct AnalogBassDrum {
    pulse_remaining_samples: i32,
    fm_pulse_remaining_samples: i32,
    pulse: f32,
    pulse_height: f32,
    pulse_lp: f32,
    fm_pulse_lp: f32,
    retrig_pulse: f32,
    lp_out: f32,
    tone_lp: f32,
    sustain_gain: f32,

    resonator: Svf,

    // Replace the resonator in "free running" (sustain) mode.
    oscillator: SineOscillator,

    // Sample rate dependent constants
    trigger_pulse_duration: i32,
    fm_pulse_duration: i32,
    pulse_decay_time: f32,
    pulse_filter_time: f32,
    retrig_pulse_duration: f32,
}

impl AnalogBassDrum {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        self.pulse_remaining_samples = 0;
        self.fm_pulse_remaining_samples = 0;
        self.pulse = 0.0;
        self.pulse_height = 0.0;
        self.pulse_lp = 0.0;
        self.fm_pulse_lp = 0.0;
        self.retrig_pulse = 0.0;
        self.lp_out = 0.0;
        self.tone_lp = 0.0;
        self.sustain_gain = 0.0;

        self.resonator.init();
        self.oscillator.init();

        // Pre-compute sample rate dependent constants
        self.trigger_pulse_duration = (1.0e-3 * sample_rate_hz) as i32;
        self.fm_pulse_duration = (6.0e-3 * sample_rate_hz) as i32;
        self.pulse_decay_time = 0.2e-3 * sample_rate_hz;
        self.pulse_filter_time = 0.1e-3 * sample_rate_hz;
        self.retrig_pulse_duration = 0.05 * sample_rate_hz;
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        sustain: bool,
        trigger: bool,
        accent: f32,
        f0: f32,
        tone: f32,
        decay: f32,
        attack_fm_amount: f32,
        self_fm_amount: f32,
        out: &mut [f32],
    ) {
        let scale = 0.001 / f0;
        let q = 1500.0 * semitones_to_ratio(decay * 80.0);
        let tone_f = f32::min(4.0 * f0 * semitones_to_ratio(tone * 108.0), 1.0);
        let exciter_leak = 0.08 * (tone + 0.25);

        if trigger {
            self.pulse_remaining_samples = self.trigger_pulse_duration;
            self.fm_pulse_remaining_samples = self.fm_pulse_duration;
            self.pulse_height = 3.0 + 7.0 * accent;
            self.lp_out = 0.0;
        }

        let mut sustain_gain =
            ParameterInterpolator::new(&mut self.sustain_gain, accent * decay, out.len());

        for out_sample in out.iter_mut() {
            // Q39 / Q40
            let mut pulse;
            if self.pulse_remaining_samples != 0 {
                self.pulse_remaining_samples -= 1;
                pulse = if self.pulse_remaining_samples != 0 {
                    self.pulse_height
                } else {
                    self.pulse_height - 1.0
                };
                self.pulse = pulse;
            } else {
                self.pulse *= 1.0 - 1.0 / self.pulse_decay_time;
                pulse = self.pulse;
            }
            if sustain {
                pulse = 0.0;
            }

            // C40 / R163 / R162 / D83
            one_pole(&mut self.pulse_lp, pulse, 1.0 / self.pulse_filter_time);
            pulse = diode((pulse - self.pulse_lp) + pulse * 0.044);

            // Q41 / Q42
            let mut fm_pulse = 0.0;
            if self.fm_pulse_remaining_samples != 0 {
                self.fm_pulse_remaining_samples -= 1;
                fm_pulse = 1.0;
                // C39 / C52
                self.retrig_pulse = if self.fm_pulse_remaining_samples != 0 {
                    0.0
                } else {
                    -0.8
                };
            } else {
                // C39 / R161
                self.retrig_pulse *= 1.0 - 1.0 / self.retrig_pulse_duration;
            }
            if sustain {
                fm_pulse = 0.0;
            }
            one_pole(
                &mut self.fm_pulse_lp,
                fm_pulse,
                1.0 / self.pulse_filter_time,
            );

            // Q43 and R170 leakage
            let punch = 0.7 + diode(10.0 * self.lp_out - 1.0);

            // Q43 / R165
            let attack_fm = self.fm_pulse_lp * 1.7 * attack_fm_amount;
            let self_fm = punch * 0.08 * self_fm_amount;
            let mut f = f0 * (1.0 + attack_fm + self_fm);
            f = f.clamp(0.0, 0.4);

            let mut resonator_out = 0.0;
            if sustain {
                self.oscillator.next_sin_cos(
                    f,
                    sustain_gain.next(),
                    &mut resonator_out,
                    &mut self.lp_out,
                );
            } else {
                self.resonator
                    .set_f_q(f, 1.0 + q * f, FrequencyApproximation::Dirty);
                self.resonator.process_dual(
                    (pulse - self.retrig_pulse * 0.2) * scale,
                    &mut resonator_out,
                    &mut self.lp_out,
                    FilterMode::BandPass,
                    FilterMode::LowPass,
                );
            }

            one_pole(
                &mut self.tone_lp,
                pulse * exciter_leak + resonator_out,
                tone_f,
            );

            *out_sample = self.tone_lp;
        }
    }
}

#[inline]
fn diode(mut x: f32) -> f32 {
    if x >= 0.0 {
        x
    } else {
        x *= 2.0;
        0.7 * x / (1.0 + x.abs())
    }
}
