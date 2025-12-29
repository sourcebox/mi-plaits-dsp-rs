//! Single waveform oscillator.
//!
//! Can optionally do audio-rate linear FM, with through-zero capabilities (negative frequencies).

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::polyblep::{
    next_blep_sample, next_integrated_blep_sample, this_blep_sample, this_integrated_blep_sample,
};

pub const MAX_FREQUENCY: f32 = 0.25;
pub const MIN_FREQUENCY: f32 = 0.000001;

#[derive(Debug, Clone)]
pub enum OscillatorShape {
    ImpulseTrain,
    Saw,
    Triangle,
    Slope,
    Square,
    SquareBright,
    SquareDark,
    SquareTriangle,
}

#[derive(Debug, Default, Clone)]
pub struct Oscillator {
    // Oscillator state.
    phase: f32,
    next_sample: f32,
    lp_state: f32,
    hp_state: f32,
    high: bool,

    // For interpolation of parameters.
    frequency: f32,
    pw: f32,
}

impl Oscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.5;
        self.next_sample = 0.0;
        self.lp_state = 1.0;
        self.hp_state = 0.0;
        self.high = true;

        self.frequency = 0.001;
        self.pw = 0.5;
    }

    #[inline]
    pub fn render(
        &mut self,
        mut frequency: f32,
        mut pw: f32,
        external_fm: Option<&[f32]>,
        out: &mut [f32],
        shape: OscillatorShape,
        through_zero_fm: bool,
    ) {
        if external_fm.is_none() {
            if !through_zero_fm {
                frequency = frequency.clamp(MIN_FREQUENCY, MAX_FREQUENCY);
            } else {
                frequency = frequency.clamp(-MAX_FREQUENCY, MAX_FREQUENCY);
            }
            pw = pw.clamp(frequency.abs() * 2.0, 1.0 - 2.0 * frequency.abs())
        }

        let mut fm = ParameterInterpolator::new(&mut self.frequency, frequency, out.len());
        let mut pwm = ParameterInterpolator::new(&mut self.pw, pw, out.len());

        let mut next_sample = self.next_sample;
        let mut external_fm_index = 0;

        for out_sample in out.iter_mut() {
            let mut this_sample = next_sample;
            next_sample = 0.0;

            let mut frequency = fm.next();
            if let Some(external_fm) = external_fm {
                frequency *= 1.0 + external_fm[external_fm_index];
                external_fm_index += 1;
                if !through_zero_fm {
                    frequency = frequency.clamp(MIN_FREQUENCY, MAX_FREQUENCY);
                } else {
                    frequency = frequency.clamp(-MAX_FREQUENCY, MAX_FREQUENCY);
                }
            }
            let mut pw = match shape {
                OscillatorShape::SquareTriangle | OscillatorShape::Triangle => 0.5,
                _ => pwm.next(),
            };
            if external_fm.is_some() {
                pw = pw.clamp(frequency.abs() * 2.0, 1.0 - 2.0 * frequency.abs());
            }
            self.phase += frequency;

            match shape {
                OscillatorShape::ImpulseTrain | OscillatorShape::Saw => {
                    if self.phase >= 1.0 {
                        self.phase -= 1.0;
                        let t = self.phase / frequency;
                        this_sample -= this_blep_sample(t);
                        next_sample -= next_blep_sample(t);
                    } else if through_zero_fm && self.phase < 0.0 {
                        let t = self.phase / frequency;
                        self.phase += 1.0;
                        this_sample += this_blep_sample(t);
                        next_sample += next_blep_sample(t);
                    }
                    next_sample += self.phase;

                    if matches!(shape, OscillatorShape::Saw) {
                        *out_sample = 2.0 * this_sample - 1.0;
                    } else {
                        self.lp_state += 0.25 * ((self.hp_state - this_sample) - self.lp_state);
                        *out_sample = 4.0 * self.lp_state;
                        self.hp_state = this_sample;
                    }
                }
                OscillatorShape::Triangle | OscillatorShape::Slope => {
                    let mut slope_up = 2.0;
                    let mut slope_down = 2.0;
                    if matches!(shape, OscillatorShape::Slope) {
                        slope_up = 1.0 / (pw);
                        slope_down = 1.0 / (1.0 - pw);
                    }
                    if self.high ^ (self.phase < pw) {
                        let t = (self.phase - pw) / frequency;
                        let mut discontinuity = (slope_up + slope_down) * frequency;
                        if through_zero_fm && frequency < 0.0 {
                            discontinuity = -discontinuity;
                        }
                        this_sample -= this_integrated_blep_sample(t) * discontinuity;
                        next_sample -= next_integrated_blep_sample(t) * discontinuity;
                        self.high = self.phase < pw;
                    }
                    if self.phase >= 1.0 {
                        self.phase -= 1.0;
                        let t = self.phase / frequency;
                        let discontinuity = (slope_up + slope_down) * frequency;
                        this_sample += this_integrated_blep_sample(t) * discontinuity;
                        next_sample += next_integrated_blep_sample(t) * discontinuity;
                        self.high = true;
                    } else if through_zero_fm && self.phase < 0.0 {
                        let t = self.phase / frequency;
                        self.phase += 1.0;
                        let discontinuity = (slope_up + slope_down) * frequency;
                        this_sample -= this_integrated_blep_sample(t) * discontinuity;
                        next_sample -= next_integrated_blep_sample(t) * discontinuity;
                        self.high = false;
                    }
                    next_sample += if self.high {
                        self.phase * slope_up
                    } else {
                        1.0 - (self.phase - pw) * slope_down
                    };
                    *out_sample = 2.0 * this_sample - 1.0;
                }
                OscillatorShape::Square
                | OscillatorShape::SquareBright
                | OscillatorShape::SquareDark
                | OscillatorShape::SquareTriangle => {
                    if self.high ^ (self.phase >= pw) {
                        let t = (self.phase - pw) / frequency;
                        let mut discontinuity = 1.0;
                        if through_zero_fm && frequency < 0.0 {
                            discontinuity = -discontinuity;
                        }
                        this_sample += this_blep_sample(t) * discontinuity;
                        next_sample += next_blep_sample(t) * discontinuity;
                        self.high = self.phase >= pw;
                    }
                    if self.phase >= 1.0 {
                        self.phase -= 1.0;
                        let t = self.phase / frequency;
                        this_sample -= this_blep_sample(t);
                        next_sample -= next_blep_sample(t);
                        self.high = false;
                    } else if through_zero_fm && self.phase < 0.0 {
                        let t = self.phase / frequency;
                        self.phase += 1.0;
                        this_sample += this_blep_sample(t);
                        next_sample += next_blep_sample(t);
                        self.high = true;
                    }
                    next_sample += if self.phase < pw { 0.0 } else { 1.0 };

                    if matches!(shape, OscillatorShape::SquareTriangle) {
                        let integrator_coefficient = frequency * 0.0625;
                        this_sample = 128.0 * (this_sample - 0.5);
                        self.lp_state += integrator_coefficient * (this_sample - self.lp_state);
                        *out_sample = self.lp_state;
                    } else if matches!(shape, OscillatorShape::SquareDark) {
                        let integrator_coefficient = frequency * 2.0;
                        this_sample = 4.0 * (this_sample - 0.5);
                        self.lp_state += integrator_coefficient * (this_sample - self.lp_state);
                        *out_sample = self.lp_state;
                    } else if matches!(shape, OscillatorShape::SquareBright) {
                        let integrator_coefficient = frequency * 2.0;
                        this_sample = 2.0 * this_sample - 1.0;
                        self.lp_state += integrator_coefficient * (this_sample - self.lp_state);
                        *out_sample = (this_sample - self.lp_state) * 0.5;
                    } else {
                        this_sample = 2.0 * this_sample - 1.0;
                        *out_sample = this_sample;
                    }
                }
            }
        }
        self.next_sample = next_sample;
    }
}
