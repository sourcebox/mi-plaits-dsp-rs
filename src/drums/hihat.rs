//! Hihat
//!
//! 808 HH, with a few extra parameters to push things to the CY territory...
//! The template parameter MetallicNoiseSource allows another kind of "metallic
//! noise" to be used, for results which are more similar to KR-55 or FM hi-hats.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use crate::oscillator::oscillator::{Oscillator, OscillatorShape};
use crate::utils::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::random;
use crate::utils::units::semitones_to_ratio;
use crate::SAMPLE_RATE;

pub enum NoiseType {
    Square,
    RingMod,
}

pub enum VcaType {
    Swing,
    Linear,
}

#[derive(Debug, Default, Clone)]
pub struct Hihat {
    envelope: f32,
    noise_clock: f32,
    noise_sample: f32,
    sustain_gain: f32,

    square_noise: SquareNoise,
    ring_mod_noise: RingModNoise,

    noise_coloration_svf: Svf,
    hpf: Svf,
}

impl Hihat {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.envelope = 0.0;
        self.noise_clock = 0.0;
        self.noise_sample = 0.0;
        self.sustain_gain = 0.0;

        self.square_noise.init();
        self.ring_mod_noise.init();
        self.noise_coloration_svf.init();
        self.hpf.init();
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
        mut noisiness: f32,
        temp_1: &mut [f32],
        temp_2: &mut [f32],
        out: &mut [f32],
        noise_type: NoiseType,
        vca_type: VcaType,
        resonance: bool,
        two_stage_envelope: bool,
    ) {
        let envelope_decay = 1.0 - 0.003 * semitones_to_ratio(-decay * 84.0);
        let cut_decay = 1.0 - 0.0025 * semitones_to_ratio(-decay * 36.0);

        if trigger {
            self.envelope = (1.5 + 0.5 * (1.0 - decay)) * (0.3 + 0.7 * accent);
        }

        // Render the metallic noise.
        match noise_type {
            NoiseType::Square => {
                self.square_noise.render(2.0 * f0, out);
            }
            NoiseType::RingMod => {
                self.ring_mod_noise.render(2.0 * f0, temp_1, temp_2, out);
            }
        }

        // Apply BPF on the metallic noise.
        let mut cutoff = 150.0 / SAMPLE_RATE * semitones_to_ratio(tone * 72.0);
        cutoff = cutoff.clamp(0.0, 16000.0 / SAMPLE_RATE);
        self.noise_coloration_svf.set_f_q(
            cutoff,
            if resonance { 3.0 + 3.0 * tone } else { 1.0 },
            FrequencyApproximation::Accurate,
        );
        self.noise_coloration_svf
            .process_buffer(out, temp_1, FilterMode::BandPass);
        out.copy_from_slice(temp_1);

        // This is not at all part of the 808 circuit! But to add more variety,
        // we add a variable amount of clocked noise to the output of the 6
        // schmitt trigger oscillators.
        noisiness *= noisiness;
        let mut noise_f = f0 * (16.0 + 16.0 * (1.0 - noisiness));
        noise_f = noise_f.clamp(0.0, 0.5);

        for out_sample in out.iter_mut() {
            self.noise_clock += noise_f;
            if self.noise_clock >= 1.0 {
                self.noise_clock -= 1.0;
                self.noise_sample = random::get_float() - 0.5;
            }
            *out_sample += noisiness * (self.noise_sample - *out_sample);
        }

        // Apply VCA.
        let mut sustain_gain =
            ParameterInterpolator::new(&mut self.sustain_gain, accent * decay, out.len());
        for sample_out in out.iter_mut() {
            self.envelope *= if self.envelope > 0.5 || !two_stage_envelope {
                envelope_decay
            } else {
                cut_decay
            };
            match vca_type {
                VcaType::Swing => {
                    *sample_out = swing_vca(
                        *sample_out,
                        if sustain {
                            sustain_gain.next()
                        } else {
                            self.envelope
                        },
                    );
                }
                VcaType::Linear => {
                    *sample_out = linear_vca(
                        *sample_out,
                        if sustain {
                            sustain_gain.next()
                        } else {
                            self.envelope
                        },
                    );
                }
            }
        }

        self.hpf
            .set_f_q(cutoff, 0.5, FrequencyApproximation::Accurate);
        self.hpf.process_buffer(out, temp_1, FilterMode::HighPass);
        out.copy_from_slice(temp_1);
    }
}

#[derive(Debug, Default, Clone)]
pub struct SquareNoise {
    phase: [u32; 6],
}

impl SquareNoise {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = [0; 6];
    }

    #[inline]
    pub fn render(&mut self, f0: f32, out: &mut [f32]) {
        let ratios = [
            // Nominal f0: 414 Hz
            1.0, 1.304, 1.466, 1.787, 1.932, 2.536,
        ];

        let mut increment = [0; 6];
        let mut phase = [0; 6];
        for i in 0..6 {
            let mut f = f0 * ratios[i];
            if f >= 0.499 {
                f = 0.499;
            }
            increment[i] = (f * 4294967296.0) as u32;
            phase[i] = self.phase[i];
        }

        for sample_out in out.iter_mut() {
            phase[0] = phase[0].wrapping_add(increment[0]);
            phase[1] = phase[1].wrapping_add(increment[1]);
            phase[2] = phase[2].wrapping_add(increment[2]);
            phase[3] = phase[3].wrapping_add(increment[3]);
            phase[4] = phase[4].wrapping_add(increment[4]);
            phase[5] = phase[5].wrapping_add(increment[5]);
            let mut noise = 0;
            noise += phase[0] >> 31;
            noise += phase[1] >> 31;
            noise += phase[2] >> 31;
            noise += phase[3] >> 31;
            noise += phase[4] >> 31;
            noise += phase[5] >> 31;
            *sample_out = 0.33 * ((noise) as f32 - 1.0);
        }

        self.phase[..6].copy_from_slice(&phase[..6]);
    }
}

#[derive(Debug, Default, Clone)]
pub struct RingModNoise {
    oscillator: [Oscillator; 6],
}

impl RingModNoise {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        for i in 0..6 {
            self.oscillator[i].init();
        }
    }

    #[inline]
    pub fn render(&mut self, f0: f32, temp_1: &mut [f32], temp_2: &mut [f32], out: &mut [f32]) {
        let ratio = f0 / (0.01 + f0);
        let f1a = 200.0 / SAMPLE_RATE * ratio;
        let f1b = 7530.0 / SAMPLE_RATE * ratio;
        let f2a = 510.0 / SAMPLE_RATE * ratio;
        let f2b = 8075.0 / SAMPLE_RATE * ratio;
        let f3a = 730.0 / SAMPLE_RATE * ratio;
        let f3b = 10500.0 / SAMPLE_RATE * ratio;

        out.fill(0.0);

        Self::render_pair(&mut self.oscillator[0..2], f1a, f1b, temp_1, temp_2, out);
        Self::render_pair(&mut self.oscillator[2..4], f2a, f2b, temp_1, temp_2, out);
        Self::render_pair(&mut self.oscillator[4..6], f3a, f3b, temp_1, temp_2, out);
    }

    #[inline]
    fn render_pair(
        osc: &mut [Oscillator],
        f1: f32,
        f2: f32,
        temp_1: &mut [f32],
        temp_2: &mut [f32],
        out: &mut [f32],
    ) {
        osc[0].render(f1, 0.5, None, temp_1, OscillatorShape::Square, false);
        osc[1].render(f2, 0.5, None, temp_2, OscillatorShape::Saw, false);

        for (sample_out, (sample_temp_1, sample_temp_2)) in
            out.iter_mut().zip(temp_1.iter().zip(temp_2.iter()))
        {
            *sample_out += *sample_temp_1 * *sample_temp_2;
        }
    }
}

#[inline]
fn swing_vca(mut s: f32, gain: f32) -> f32 {
    s *= if s > 0.0 { 4.0 } else { 0.1 };
    s = s / (1.0 + s.abs());

    (s + 0.1) * gain
}

#[inline]
fn linear_vca(s: f32, gain: f32) -> f32 {
    s * gain
}
