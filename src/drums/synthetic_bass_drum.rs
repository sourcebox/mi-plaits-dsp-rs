//! Naive bass drum model (modulated oscillator with FM + envelope).
//! Inadvertently 909-ish.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use crate::oscillator::sine_oscillator::sine;
use crate::utils::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::random;
use crate::utils::units::semitones_to_ratio;
use crate::utils::{one_pole, scaled_smoothing_coefficient, slope, sqrt, REFERENCE_SAMPLE_RATE};

#[derive(Debug, Default, Clone)]
pub struct SyntheticBassDrum {
    sample_rate_hz: f32,
    f0: f32,
    phase: f32,
    phase_noise: f32,

    fm: f32,
    fm_lp: f32,
    body_env: f32,
    body_env_lp: f32,
    transient_env: f32,
    transient_env_lp: f32,

    sustain_gain: f32,

    tone_lp: f32,

    click: SyntheticBassDrumClick,
    noise: SyntheticBassDrumAttackNoise,

    body_env_pulse_width: i32,
    fm_pulse_width: i32,

    // Sample rate dependent constants
    sr_ratio: f32,
    phase_noise_coefficient: f32,
    phase_noise_gain: f32,
    envelope_lp_coefficient: f32,
}

impl SyntheticBassDrum {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        self.sample_rate_hz = sample_rate_hz;
        self.phase = 0.0;
        self.phase_noise = 0.0;
        self.f0 = 0.0;
        self.fm = 0.0;
        self.fm_lp = 0.0;
        self.body_env_lp = 0.0;
        self.body_env = 0.0;
        self.body_env_pulse_width = 0;
        self.fm_pulse_width = 0;
        self.tone_lp = 0.0;
        self.sustain_gain = 0.0;

        self.click.init(sample_rate_hz);
        self.noise.init(sample_rate_hz);

        // Keep smoothing time constants in seconds at any sample rate.
        let rate_ratio = REFERENCE_SAMPLE_RATE / sample_rate_hz;
        self.sr_ratio = sample_rate_hz / REFERENCE_SAMPLE_RATE;
        self.phase_noise_coefficient = scaled_smoothing_coefficient(0.002, rate_ratio);
        // Compensate the white noise spectral density so the low-passed phase
        // noise keeps the same variance at any sample rate.
        self.phase_noise_gain = sqrt(sample_rate_hz / REFERENCE_SAMPLE_RATE);
        self.envelope_lp_coefficient = scaled_smoothing_coefficient(0.1, rate_ratio);
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
        mut decay: f32,
        mut dirtiness: f32,
        fm_envelope_amount: f32,
        mut fm_envelope_decay: f32,
        out: &mut [f32],
    ) {
        decay *= decay;
        fm_envelope_decay *= fm_envelope_decay;

        let mut f0_mod = ParameterInterpolator::new(&mut self.f0, f0, out.len());

        // The dirtiness limit is defined in terms of the normalized
        // frequency at the reference rate.
        dirtiness *= f32::max(1.0 - 8.0 * f0 * self.sr_ratio, 0.0);

        let fm_decay = 1.0 - 1.0 / (0.008 * (1.0 + fm_envelope_decay * 4.0) * self.sample_rate_hz);

        let body_env_decay =
            1.0 - 1.0 / (0.02 * self.sample_rate_hz) * semitones_to_ratio(-decay * 60.0);
        let transient_env_decay = 1.0 - 1.0 / (0.005 * self.sample_rate_hz);
        let tone_f = f32::min(4.0 * f0 * semitones_to_ratio(tone * 108.0), 1.0);
        let transient_level = tone;

        if trigger {
            self.fm = 1.0;
            self.body_env = 0.3 + 0.7 * accent;
            self.transient_env = self.body_env;
            self.body_env_pulse_width = (self.sample_rate_hz * 0.001) as i32;
            self.fm_pulse_width = (self.sample_rate_hz * 0.0013) as i32;
        }

        let mut sustain_gain =
            ParameterInterpolator::new(&mut self.sustain_gain, accent * decay, out.len());

        for out_sample in out.iter_mut() {
            one_pole(
                &mut self.phase_noise,
                (random::get_float() - 0.5) * self.phase_noise_gain,
                self.phase_noise_coefficient,
            );

            let mut mix = 0.0;

            if sustain {
                self.phase += f0_mod.next();
                if self.phase >= 1.0 {
                    self.phase -= 1.0;
                }
                let body = Self::distorted_sine(self.phase, self.phase_noise, dirtiness);
                mix -= Self::transistor_vca(body, sustain_gain.next());
            } else {
                if self.fm_pulse_width != 0 {
                    self.fm_pulse_width -= 1;
                    self.phase = 0.25;
                } else {
                    self.fm *= fm_decay;
                    let fm = 1.0 + fm_envelope_amount * 3.5 * self.fm_lp;
                    self.phase += f32::min(f0_mod.next() * fm, 0.5);
                    if self.phase >= 1.0 {
                        self.phase -= 1.0;
                    }
                }

                if self.body_env_pulse_width != 0 {
                    self.body_env_pulse_width -= 1;
                } else {
                    self.body_env *= body_env_decay;
                    self.transient_env *= transient_env_decay;
                }

                let envelope_lp_f = self.envelope_lp_coefficient;
                one_pole(&mut self.body_env_lp, self.body_env, envelope_lp_f);
                one_pole(
                    &mut self.transient_env_lp,
                    self.transient_env,
                    envelope_lp_f,
                );
                one_pole(&mut self.fm_lp, self.fm, envelope_lp_f);

                let body = Self::distorted_sine(self.phase, self.phase_noise, dirtiness);
                let transient = self.click.process(if self.body_env_pulse_width != 0 {
                    0.0
                } else {
                    1.0
                }) + self.noise.render();

                mix -= Self::transistor_vca(body, self.body_env_lp);
                mix -= transient * self.transient_env_lp * transient_level;
            }

            one_pole(&mut self.tone_lp, mix, tone_f);
            *out_sample = self.tone_lp;
        }
    }

    #[inline]
    fn distorted_sine(mut phase: f32, phase_noise: f32, dirtiness: f32) -> f32 {
        phase += phase_noise * dirtiness;
        let phase_integral = phase as usize;
        let phase_fractional = phase - (phase_integral as f32);
        phase = phase_fractional;
        let triangle = (if phase < 0.5 { phase } else { 1.0 - phase }) * 4.0 - 1.0;
        let sine_ = 2.0 * triangle / (1.0 + triangle.abs());
        let clean_sine = sine(phase + 0.75);

        sine_ + (1.0 - dirtiness) * (clean_sine - sine_)
    }

    #[inline]
    fn transistor_vca(mut s: f32, gain: f32) -> f32 {
        s = (s - 0.6) * gain;

        3.0 * s / (2.0 + s.abs()) + gain * 0.3
    }
}

#[derive(Debug, Default, Clone)]
pub struct SyntheticBassDrumClick {
    sample_rate_hz: f32,
    lp: f32,
    hp: f32,
    filter: Svf,

    // Sample rate dependent constants
    slope_up: f32,
    slope_down: f32,
    hp_coefficient: f32,
}

impl SyntheticBassDrumClick {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        self.sample_rate_hz = sample_rate_hz;
        self.lp = 0.0;
        self.hp = 0.0;
        self.filter.init();
        self.filter.set_f_q(
            5000.0 / self.sample_rate_hz,
            2.0,
            FrequencyApproximation::Fast,
        );

        // Keep the click attack/release times constant in seconds.
        let rate_ratio = REFERENCE_SAMPLE_RATE / sample_rate_hz;
        self.slope_up = (0.5 * rate_ratio).min(1.0);
        self.slope_down = scaled_smoothing_coefficient(0.1, rate_ratio);
        self.hp_coefficient = scaled_smoothing_coefficient(0.04, rate_ratio);
    }

    #[inline]
    pub fn process(&mut self, in_: f32) -> f32 {
        slope(&mut self.lp, in_, self.slope_up, self.slope_down);
        one_pole(&mut self.hp, self.lp, self.hp_coefficient);

        self.filter.process(self.lp - self.hp, FilterMode::LowPass)
    }
}

#[derive(Debug, Default, Clone)]
pub struct SyntheticBassDrumAttackNoise {
    lp: f32,
    hp: f32,

    // Sample rate dependent constants
    lp_coefficient: f32,
    hp_coefficient: f32,
    noise_gain: f32,
}

impl SyntheticBassDrumAttackNoise {
    pub fn new() -> Self {
        Self {
            lp: 0.0,
            hp: 0.0,
            lp_coefficient: 0.05,
            hp_coefficient: 0.005,
            noise_gain: 1.0,
        }
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        self.lp = 0.0;
        self.hp = 0.0;

        // Keep the noise band constant in Hz, and compensate the white noise
        // spectral density so the level stays the same at any sample rate.
        let rate_ratio = REFERENCE_SAMPLE_RATE / sample_rate_hz;
        self.lp_coefficient = scaled_smoothing_coefficient(0.05, rate_ratio);
        self.hp_coefficient = scaled_smoothing_coefficient(0.005, rate_ratio);
        self.noise_gain = sqrt(sample_rate_hz / REFERENCE_SAMPLE_RATE);
    }

    #[inline]
    pub fn render(&mut self) -> f32 {
        let sample = random::get_float() * self.noise_gain;
        one_pole(&mut self.lp, sample, self.lp_coefficient);
        one_pole(&mut self.hp, self.lp, self.hp_coefficient);

        self.lp - self.hp
    }
}
