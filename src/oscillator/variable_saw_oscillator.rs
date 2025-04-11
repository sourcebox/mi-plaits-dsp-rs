//! Saw with variable slope or notch

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::oscillator::oscillator::MAX_FREQUENCY;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::polyblep::{
    next_blep_sample, next_integrated_blep_sample, this_blep_sample, this_integrated_blep_sample,
};

const VARIABLE_SAW_NOTCH_DEPTH: f32 = 0.2;

#[derive(Debug, Default)]
pub struct VariableSawOscillator {
    // Oscillator state.
    phase: f32,
    next_sample: f32,
    previous_pw: f32,
    high: bool,

    // For interpolation of parameters.
    frequency: f32,
    pw: f32,
    waveshape: f32,
}

impl VariableSawOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.next_sample = 0.0;
        self.previous_pw = 0.5;
        self.high = false;

        self.frequency = 0.01;
        self.pw = 0.5;
        self.waveshape = 0.0;
    }

    #[inline]
    pub fn render(&mut self, mut frequency: f32, mut pw: f32, waveshape: f32, out: &mut [f32]) {
        if frequency >= MAX_FREQUENCY {
            frequency = MAX_FREQUENCY;
        }

        if frequency >= 0.25 {
            pw = 0.5;
        } else {
            pw = pw.clamp(frequency * 2.0, 1.0 - 2.0 * frequency);
        }

        let mut fm = ParameterInterpolator::new(&mut self.frequency, frequency, out.len());
        let mut pwm = ParameterInterpolator::new(&mut self.pw, pw, out.len());
        let mut waveshape_modulation =
            ParameterInterpolator::new(&mut self.waveshape, waveshape, out.len());

        let mut next_sample = self.next_sample;

        for out_sample in out.iter_mut() {
            let mut this_sample = next_sample;
            next_sample = 0.0;

            let frequency = fm.next();
            let pw = pwm.next();
            let waveshape = waveshape_modulation.next();
            let triangle_amount = waveshape;
            let notch_amount = 1.0 - waveshape;
            let slope_up = 1.0 / (pw);
            let slope_down = 1.0 / (1.0 - pw);

            self.phase += frequency;

            if !self.high && self.phase >= pw {
                let triangle_step = (slope_up + slope_down) * frequency * triangle_amount;
                let notch = (VARIABLE_SAW_NOTCH_DEPTH + 1.0 - pw) * notch_amount;
                let t = (self.phase - pw) / (self.previous_pw - pw + frequency);
                this_sample += notch * this_blep_sample(t);
                next_sample += notch * next_blep_sample(t);
                this_sample -= triangle_step * this_integrated_blep_sample(t);
                next_sample -= triangle_step * next_integrated_blep_sample(t);
                self.high = true;
            } else if self.phase >= 1.0 {
                self.phase -= 1.0;
                let triangle_step = (slope_up + slope_down) * frequency * triangle_amount;
                let notch = (VARIABLE_SAW_NOTCH_DEPTH + 1.0) * notch_amount;
                let t = self.phase / frequency;
                this_sample -= notch * this_blep_sample(t);
                next_sample -= notch * next_blep_sample(t);
                this_sample += triangle_step * this_integrated_blep_sample(t);
                next_sample += triangle_step * next_integrated_blep_sample(t);
                self.high = false;
            }

            next_sample += compute_naive_sample(
                self.phase,
                pw,
                slope_up,
                slope_down,
                triangle_amount,
                notch_amount,
            );
            self.previous_pw = pw;

            *out_sample = (2.0 * this_sample - 1.0) / (1.0 + VARIABLE_SAW_NOTCH_DEPTH);
        }

        self.next_sample = next_sample;
    }
}

#[inline]
fn compute_naive_sample(
    phase: f32,
    pw: f32,
    slope_up: f32,
    slope_down: f32,
    triangle_amount: f32,
    notch_amount: f32,
) -> f32 {
    let notch_saw = if phase < pw {
        phase
    } else {
        1.0 + VARIABLE_SAW_NOTCH_DEPTH
    };
    let triangle = if phase < pw {
        phase * slope_up
    } else {
        1.0 - (phase - pw) * slope_down
    };

    notch_saw * notch_amount + triangle * triangle_amount
}
