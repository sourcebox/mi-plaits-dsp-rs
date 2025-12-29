//! 808 snare drum model, revisited.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::oscillator::sine_oscillator::SineOscillator;
use crate::utils::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::random;
use crate::utils::units::semitones_to_ratio;
use crate::utils::{one_pole, soft_clip};

const NUM_MODES: usize = 5;

#[derive(Debug, Default, Clone)]
pub struct AnalogSnareDrum {
    pulse_remaining_samples: i32,
    pulse: f32,
    pulse_height: f32,
    pulse_lp: f32,
    noise_envelope: f32,
    sustain_gain: f32,

    resonator: [Svf; NUM_MODES],
    noise_filter: Svf,

    // Replace the resonators in "free running" (sustain) mode.
    oscillator: [SineOscillator; NUM_MODES],

    // Sample rate dependent constants
    trigger_pulse_duration: i32,
    pulse_decay_time: i32,
}

impl AnalogSnareDrum {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        self.pulse_remaining_samples = 0;
        self.pulse = 0.0;
        self.pulse_height = 0.0;
        self.pulse_lp = 0.0;
        self.noise_envelope = 0.0;
        self.sustain_gain = 0.0;

        for i in 0..NUM_MODES {
            self.resonator[i].init();
            self.oscillator[i].init();
        }
        self.noise_filter.init();

        // Pre-compute sample rate dependent constants
        self.trigger_pulse_duration = (1.0e-3 * sample_rate_hz) as i32;
        self.pulse_decay_time = (0.1e-3 * sample_rate_hz) as i32;
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        sustain: bool,
        trigger: bool,
        accent: f32,
        f0: f32,
        mut tone: f32,
        decay: f32,
        mut snappy: f32,
        out: &mut [f32],
    ) {
        let decay_xt = decay * (1.0 + decay * (decay - 1.0));
        let q = 2000.0 * semitones_to_ratio(decay_xt * 84.0);
        let noise_envelope_decay =
            1.0 - 0.0017 * semitones_to_ratio(-decay * (50.0 + snappy * 10.0));
        let exciter_leak = snappy * (2.0 - snappy) * 0.1;

        snappy = (snappy * 1.1 - 0.05).clamp(0.0, 1.0);

        if trigger {
            self.pulse_remaining_samples = self.trigger_pulse_duration;
            self.pulse_height = 3.0 + 7.0 * accent;
            self.noise_envelope = 2.0;
        }

        const MODE_FREQUENCIES: [f32; NUM_MODES] = [1.00, 2.00, 3.18, 4.16, 5.62];

        let mut f: [f32; NUM_MODES] = [0.0; NUM_MODES];
        let mut gain: [f32; NUM_MODES] = [0.0; NUM_MODES];

        for i in 0..NUM_MODES {
            f[i] = f32::min(f0 * MODE_FREQUENCIES[i], 0.499);
            self.resonator[i].set_f_q(
                f[i],
                1.0 + f[i] * (if i == 0 { q } else { q * 0.25 }),
                FrequencyApproximation::Fast,
            );
        }

        if tone < 0.666667 {
            // 808-style (2 modes)
            tone *= 1.5;
            gain[0] = 1.5 + (1.0 - tone) * (1.0 - tone) * 4.5;
            gain[1] = 2.0 * tone + 0.15;
            for v in gain.iter_mut().take(NUM_MODES).skip(2) {
                *v = 0.0;
            }
        } else {
            // What the 808 could have been if there were extra modes!
            tone = (tone - 0.666667) * 3.0;
            gain[0] = 1.5 - tone * 0.5;
            gain[1] = 2.15 - tone * 0.7;
            for v in gain.iter_mut().take(NUM_MODES).skip(2) {
                *v = tone;
                tone *= tone;
            }
        }

        let f_noise = (f0 * 16.0).clamp(0.0, 0.499);
        self.noise_filter
            .set_f_q(f_noise, 1.0 + f_noise * 1.5, FrequencyApproximation::Fast);

        let mut sustain_gain =
            ParameterInterpolator::new(&mut self.sustain_gain, accent * decay, out.len());

        for out_sample in out.iter_mut() {
            // Q45 / Q46
            let pulse;
            if self.pulse_remaining_samples != 0 {
                self.pulse_remaining_samples -= 1;
                pulse = if self.pulse_remaining_samples != 0 {
                    self.pulse_height
                } else {
                    self.pulse_height - 1.0
                };
                self.pulse = pulse;
            } else {
                self.pulse *= 1.0 - 1.0 / (self.pulse_decay_time as f32);
                pulse = self.pulse;
            }

            let sustain_gain_value = sustain_gain.next();

            // R189 / C57 / R190 + C58 / C59 / R197 / R196 / IC14
            one_pole(&mut self.pulse_lp, pulse, 0.75);

            let mut shell = 0.0;
            for i in 0..NUM_MODES {
                let excitation = if i == 0 {
                    (pulse - self.pulse_lp) + 0.006 * pulse
                } else {
                    0.026 * pulse
                };
                shell += gain[i]
                    * (if sustain {
                        self.oscillator[i].next(f[i]) * sustain_gain_value * 0.25
                    } else {
                        self.resonator[i].process(excitation, FilterMode::BandPass)
                            + excitation * exciter_leak
                    });
            }
            shell = soft_clip(shell);

            // C56 / R194 / Q48 / C54 / R188 / D54
            let mut noise = 2.0 * random::get_float() - 1.0;
            if noise < 0.0 {
                noise = 0.0;
            }
            self.noise_envelope *= noise_envelope_decay;
            noise *= (if sustain {
                sustain_gain_value
            } else {
                self.noise_envelope
            }) * snappy
                * 2.0;

            // C66 / R201 / C67 / R202 / R203 / Q49
            noise = self.noise_filter.process(noise, FilterMode::BandPass);

            // IC13
            *out_sample = noise + shell * (1.0 - snappy);
        }
    }
}
