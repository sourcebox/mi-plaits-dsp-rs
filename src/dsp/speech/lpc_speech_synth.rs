//! LPC10 speech synth.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::dsp::resources::{LUT_LPC_EXCITATION_PULSE, LUT_LPC_EXCITATION_PULSE_SIZE};
use crate::stmlib::dsp::polyblep::{next_blep_sample, this_blep_sample};
use crate::stmlib::utils::random;

pub const LPC_ORDER: usize = 10;
pub const LPC_SPEECH_SYNTH_DEFAULT_F0: f32 = 100.0;

#[derive(Debug, Default)]
pub struct LpcSpeechSynthFrame {
    // 14 bytes.
    pub energy: u8,
    pub period: u8,
    pub k0: i16,
    pub k1: i16,
    pub k2: i8,
    pub k3: i8,
    pub k4: i8,
    pub k5: i8,
    pub k6: i8,
    pub k7: i8,
    pub k8: i8,
    pub k9: i8,
}

impl LpcSpeechSynthFrame {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        energy: u8,
        period: u8,
        k0: i16,
        k1: i16,
        k2: i8,
        k3: i8,
        k4: i8,
        k5: i8,
        k6: i8,
        k7: i8,
        k8: i8,
        k9: i8,
    ) -> Self {
        Self {
            energy,
            period,
            k0,
            k1,
            k2,
            k3,
            k4,
            k5,
            k6,
            k7,
            k8,
            k9,
        }
    }
}

#[derive(Debug, Default)]
pub struct LpcSpeechSynth {
    phase: f32,
    frequency: f32,
    noise_energy: f32,
    pulse_energy: f32,

    next_sample: f32,
    excitation_pulse_sample_index: usize,

    k: [f32; LPC_ORDER],
    s: [f32; LPC_ORDER + 1],
}

impl LpcSpeechSynth {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.frequency = 0.0125;
        self.noise_energy = 0.0;
        self.pulse_energy = 0.0;

        self.next_sample = 0.0;
        self.excitation_pulse_sample_index = 0;

        self.k.fill(0.0);
        self.s.fill(0.0);
    }

    #[inline]
    pub fn render(
        &mut self,
        prosody_amount: f32,
        pitch_shift: f32,
        excitation: &mut [f32],
        output: &mut [f32],
    ) {
        let base_f0 = LPC_SPEECH_SYNTH_DEFAULT_F0 / 8000.0;
        let d = self.frequency - base_f0;
        let mut f = (base_f0 + d * prosody_amount) * pitch_shift;
        f = f.clamp(0.0, 0.5);

        let mut next_sample = self.next_sample;

        for (excitation_sample, output_sample) in excitation.iter_mut().zip(output.iter_mut()) {
            self.phase += f;

            let mut this_sample = next_sample;
            next_sample = 0.0;

            if self.phase >= 1.0 {
                self.phase -= 1.0;
                let reset_time = self.phase / f;
                let reset_sample = (32.0 * reset_time) as usize;

                let mut discontinuity = 0.0;
                if self.excitation_pulse_sample_index < LUT_LPC_EXCITATION_PULSE_SIZE {
                    self.excitation_pulse_sample_index -= reset_sample;
                    let s = LUT_LPC_EXCITATION_PULSE[self.excitation_pulse_sample_index];
                    discontinuity = (s as f32) / 128.0 * self.pulse_energy;
                }

                this_sample += -discontinuity * this_blep_sample(reset_time);
                next_sample += -discontinuity * next_blep_sample(reset_time);

                self.excitation_pulse_sample_index = reset_sample;
            }

            let mut e: [f32; 11] = [0.0; 11];

            e[10] = if random::get_sample() > 0 {
                self.noise_energy
            } else {
                -self.noise_energy
            };

            if self.excitation_pulse_sample_index < LUT_LPC_EXCITATION_PULSE_SIZE {
                let s = LUT_LPC_EXCITATION_PULSE[self.excitation_pulse_sample_index];
                next_sample += (s as f32) / 128.0 * self.pulse_energy;
                self.excitation_pulse_sample_index += 32;
            }

            e[10] += this_sample;
            e[10] *= 1.5;

            e[9] = e[10] - self.k[9] * self.s[9];
            e[8] = e[9] - self.k[8] * self.s[8];
            e[7] = e[8] - self.k[7] * self.s[7];
            e[6] = e[7] - self.k[6] * self.s[6];
            e[5] = e[6] - self.k[5] * self.s[5];
            e[4] = e[5] - self.k[4] * self.s[4];
            e[3] = e[4] - self.k[3] * self.s[3];
            e[2] = e[3] - self.k[2] * self.s[2];
            e[1] = e[2] - self.k[1] * self.s[1];
            e[0] = e[1] - self.k[0] * self.s[0];

            e[0] = e[0].clamp(-2.0, 2.0);

            self.s[9] = self.s[8] + self.k[8] * e[8];
            self.s[8] = self.s[7] + self.k[7] * e[7];
            self.s[7] = self.s[6] + self.k[6] * e[6];
            self.s[6] = self.s[5] + self.k[5] * e[5];
            self.s[5] = self.s[4] + self.k[4] * e[4];
            self.s[4] = self.s[3] + self.k[3] * e[3];
            self.s[3] = self.s[2] + self.k[2] * e[2];
            self.s[2] = self.s[1] + self.k[1] * e[1];
            self.s[1] = self.s[0] + self.k[0] * e[0];
            self.s[0] = e[0];

            *excitation_sample = e[10];
            *output_sample = e[0];
        }

        self.next_sample = next_sample;
    }

    #[inline]
    pub fn play_frame(&mut self, frames: &[LpcSpeechSynthFrame], frame: f32, interpolate: bool) {
        let mut frame_integral = frame as usize;
        let mut frame_fractional = frame - (frame_integral as f32);

        if !interpolate {
            frame_fractional = 0.0;
        }

        frame_integral = frame_integral.clamp(0, frames.len() - 2);

        self.play_frame_blend(
            &frames[frame_integral],
            &frames[frame_integral + 1],
            frame_fractional,
        );
    }

    #[inline]
    fn play_frame_blend(&mut self, f1: &LpcSpeechSynthFrame, f2: &LpcSpeechSynthFrame, blend: f32) {
        let frequency_1 = if f1.period == 0 {
            self.frequency
        } else {
            1.0 / (f1.period as f32)
        };
        let frequency_2 = if f2.period == 0 {
            self.frequency
        } else {
            1.0 / (f2.period as f32)
        };
        self.frequency = frequency_1 + (frequency_2 - frequency_1) * blend;

        let energy_1 = (f1.energy as f32) / 256.0;
        let energy_2 = (f2.energy as f32) / 256.0;
        let noise_energy_1 = if f1.period == 0 { energy_1 } else { 0.0 };
        let noise_energy_2 = if f2.period == 0 { energy_2 } else { 0.0 };
        self.noise_energy = noise_energy_1 + (noise_energy_2 - noise_energy_1) * blend;

        let pulse_energy_1 = if f1.period != 0 { energy_1 } else { 0.0 };
        let pulse_energy_2 = if f2.period != 0 { energy_2 } else { 0.0 };
        self.pulse_energy = pulse_energy_1 + (pulse_energy_2 - pulse_energy_1) * blend;

        self.k[0] = blend_coefficient(f1.k0, f2.k0, blend, 32768);
        self.k[1] = blend_coefficient(f1.k1, f2.k1, blend, 32768);
        self.k[2] = blend_coefficient(f1.k2, f2.k2, blend, 128);
        self.k[3] = blend_coefficient(f1.k3, f2.k3, blend, 128);
        self.k[4] = blend_coefficient(f1.k4, f2.k4, blend, 128);
        self.k[5] = blend_coefficient(f1.k5, f2.k5, blend, 128);
        self.k[6] = blend_coefficient(f1.k6, f2.k6, blend, 128);
        self.k[7] = blend_coefficient(f1.k7, f2.k7, blend, 128);
        self.k[8] = blend_coefficient(f1.k8, f2.k8, blend, 128);
        self.k[9] = blend_coefficient(f1.k9, f2.k9, blend, 128);
    }
}

#[inline]
fn blend_coefficient<T: Into<f32>>(a: T, b: T, blend: f32, scale: i32) -> f32 {
    let a_f = a.into() / scale as f32;
    let b_f = b.into() / scale as f32;

    a_f + (b_f - a_f) * blend
}
