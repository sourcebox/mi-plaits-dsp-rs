//! DX7-compatible LFO.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::dx_units::{lfo_delay, lfo_frequency, pitch_mod_sensitivity};
use super::patch::ModulationParameters;
use crate::dsp::oscillator::sine_oscillator::sine;
use crate::stmlib::utils::random;

#[derive(Debug, Default)]
pub enum Waveform {
    #[default]
    Triangle,

    RampDown,
    RampUp,
    Square,
    Sine,
    SAndH,
}

impl<T> From<T> for Waveform
where
    T: Into<usize>,
{
    fn from(value: T) -> Self {
        match value.into() {
            1 => Waveform::RampDown,
            2 => Waveform::RampUp,
            3 => Waveform::Square,
            4 => Waveform::Sine,
            5 => Waveform::SAndH,
            _ => Waveform::Triangle,
        }
    }
}

#[derive(Debug, Default)]
pub struct Lfo {
    phase: f32,
    frequency: f32,
    delay_phase: f32,
    delay_increment: [f32; 2],
    value: f32,

    random_value: f32,
    one_hz: f32,

    amp_mod_depth: f32,
    pitch_mod_depth: f32,

    waveform: Waveform,
    reset_phase: bool,

    phase_integral: i32,
}

impl Lfo {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn init(&mut self, sample_rate: f32) {
        self.phase = 0.0;
        self.frequency = 0.1;
        self.delay_phase = 0.0;
        self.delay_increment[0] = 0.1;
        self.delay_increment[1] = 0.1;
        self.random_value = 0.0;
        self.value = 0.0;

        self.one_hz = 1.0 / sample_rate;

        self.amp_mod_depth = 0.0;
        self.pitch_mod_depth = 0.0;

        self.waveform = Waveform::Triangle;
        self.reset_phase = false;

        self.phase_integral = 0;
    }

    #[inline]
    pub fn set(&mut self, modulations: &ModulationParameters) {
        self.frequency = lfo_frequency(modulations.rate) * self.one_hz;

        lfo_delay(modulations.delay, &mut self.delay_increment);
        self.delay_increment[0] *= self.one_hz;
        self.delay_increment[1] *= self.one_hz;

        self.waveform = Waveform::from(modulations.waveform);
        self.reset_phase = modulations.reset_phase != 0;

        self.amp_mod_depth = modulations.amp_mod_depth as f32 * 0.01;

        self.pitch_mod_depth = modulations.pitch_mod_depth as f32
            * 0.01
            * pitch_mod_sensitivity(modulations.pitch_mod_sensitivity);
    }

    #[inline]
    pub fn reset(&mut self) {
        if self.reset_phase {
            self.phase = 0.0;
        }

        self.delay_phase = 0.0;
    }

    #[inline]
    pub fn step(&mut self, scale: f32) {
        self.phase += scale * self.frequency;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.random_value = random::get_float();
        }

        self.value = self.value();
        self.delay_phase +=
            scale * self.delay_increment[if self.delay_phase < 0.5 { 0 } else { 1 }];

        if self.delay_phase >= 1.0 {
            self.delay_phase = 1.0;
        }
    }

    #[inline]
    pub fn scrub(&mut self, mut sample: f32) {
        let phase = sample * self.frequency;
        let phase_integral = phase as i32;
        let phase_fractional = phase - (phase_integral as f32);

        self.phase = phase_fractional;

        if phase_integral != self.phase_integral {
            self.phase_integral = phase_integral;
            self.random_value = random::get_float();
        }

        self.value = self.value();

        self.delay_phase = sample * self.delay_increment[0];

        if self.delay_phase > 0.5 {
            sample -= 0.5 / self.delay_increment[0];
            self.delay_phase = 0.5 + sample * self.delay_increment[1];
            if self.delay_phase >= 1.0 {
                self.delay_phase = 1.0;
            }
        }
    }

    #[inline]
    pub fn value(&self) -> f32 {
        match self.waveform {
            Waveform::Triangle => {
                2.0 * (if self.phase < 0.5 {
                    0.5 - self.phase
                } else {
                    self.phase - 0.5
                })
            }
            Waveform::RampDown => 1.0 - self.phase,
            Waveform::RampUp => self.phase,
            Waveform::Square => {
                if self.phase < 0.5 {
                    0.0
                } else {
                    1.0
                }
            }
            Waveform::Sine => 0.5 + 0.5 * sine(self.phase + 0.5),
            Waveform::SAndH => self.random_value,
        }
    }

    #[inline]
    pub fn delay_ramp(&self) -> f32 {
        if self.delay_phase < 0.5 {
            0.0
        } else {
            (self.delay_phase - 0.5) * 2.0
        }
    }

    #[inline]
    pub fn pitch_mod(&self) -> f32 {
        (self.value - 0.5) * self.delay_ramp() * self.pitch_mod_depth
    }

    #[inline]
    pub fn amp_mod(&self) -> f32 {
        (1.0 - self.value) * self.delay_ramp() * self.amp_mod_depth
    }
}
