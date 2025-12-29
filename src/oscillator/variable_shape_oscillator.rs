//! Variable waveform oscillator
//!
//! Continuously variable waveform: triangle > saw > square. Both square and
//! triangle have variable slope / pulse-width. Additionally, the phase resets
//! can be locked to a master frequency.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use crate::oscillator::oscillator::MAX_FREQUENCY;
use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::polyblep::{
    next_blep_sample, next_integrated_blep_sample, this_blep_sample, this_integrated_blep_sample,
};

#[derive(Debug, Default, Clone)]
pub struct VariableShapeOscillator {
    // Oscillator state.
    master_phase: f32,
    slave_phase: f32,
    next_sample: f32,
    previous_pw: f32,
    high: bool,

    // For interpolation of parameters.
    master_frequency: f32,
    slave_frequency: f32,
    pw: f32,
    waveshape: f32,
    phase_modulation: f32,
}

impl VariableShapeOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.master_phase = 0.0;
        self.slave_phase = 0.0;
        self.next_sample = 0.0;
        self.previous_pw = 0.5;
        self.high = false;

        self.master_frequency = 0.0;
        self.slave_frequency = 0.01;
        self.pw = 0.5;
        self.waveshape = 0.0;
        self.phase_modulation = 0.0;
    }

    pub fn set_master_phase(&mut self, phase: f32) {
        self.master_phase = phase;
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        mut master_frequency: f32,
        mut frequency: f32,
        mut pw: f32,
        waveshape: f32,
        phase_modulation_amount: f32,
        out: &mut [f32],
        enable_sync: bool,
        output_phase: bool,
    ) {
        if master_frequency >= MAX_FREQUENCY {
            master_frequency = MAX_FREQUENCY;
        }
        if frequency >= MAX_FREQUENCY {
            frequency = MAX_FREQUENCY;
        }

        if frequency >= 0.25 {
            pw = 0.5;
        } else {
            pw = pw.clamp(frequency * 2.0, 1.0 - 2.0 * frequency);
        }

        let mut master_fm =
            ParameterInterpolator::new(&mut self.master_frequency, master_frequency, out.len());
        let mut fm = ParameterInterpolator::new(&mut self.slave_frequency, frequency, out.len());
        let mut pwm = ParameterInterpolator::new(&mut self.pw, pw, out.len());
        let mut waveshape_modulation =
            ParameterInterpolator::new(&mut self.waveshape, waveshape, out.len());
        let mut phase_modulation = ParameterInterpolator::new(
            &mut self.phase_modulation,
            phase_modulation_amount,
            out.len(),
        );

        let mut next_sample = self.next_sample;

        for out_sample in out.iter_mut() {
            let mut reset = false;
            let mut transition_during_reset = false;
            let mut reset_time = 0.0;

            let mut this_sample = next_sample;
            next_sample = 0.0;

            let master_frequency = master_fm.next();
            let slave_frequency = fm.next();
            let pw = pwm.next();
            let waveshape = waveshape_modulation.next();
            let square_amount = f32::max(waveshape - 0.5, 0.0) * 2.0;
            let triangle_amount = f32::max(1.0 - waveshape * 2.0, 0.0);
            let slope_up = 1.0 / (pw);
            let slope_down = 1.0 / (1.0 - pw);

            if enable_sync {
                self.master_phase += master_frequency;
                if self.master_phase >= 1.0 {
                    self.master_phase -= 1.0;
                    reset_time = self.master_phase / master_frequency;

                    let mut slave_phase_at_reset =
                        self.slave_phase + (1.0 - reset_time) * slave_frequency;
                    reset = true;
                    if slave_phase_at_reset >= 1.0 {
                        slave_phase_at_reset -= 1.0;
                        transition_during_reset = true;
                    }
                    if !self.high && slave_phase_at_reset >= pw {
                        transition_during_reset = true;
                    }
                    let value = compute_naive_sample(
                        slave_phase_at_reset,
                        pw,
                        slope_up,
                        slope_down,
                        triangle_amount,
                        square_amount,
                    );
                    this_sample -= value * this_blep_sample(reset_time);
                    next_sample -= value * next_blep_sample(reset_time);
                }
            }

            self.slave_phase += slave_frequency;

            if transition_during_reset || !reset {
                loop {
                    if !self.high {
                        if self.slave_phase < pw {
                            break;
                        }
                        let t = (self.slave_phase - pw) / (self.previous_pw - pw + slave_frequency);
                        let mut triangle_step = (slope_up + slope_down) * slave_frequency;
                        triangle_step *= triangle_amount;

                        this_sample += square_amount * this_blep_sample(t);
                        next_sample += square_amount * next_blep_sample(t);
                        this_sample -= triangle_step * this_integrated_blep_sample(t);
                        next_sample -= triangle_step * next_integrated_blep_sample(t);
                        self.high = true;
                    }

                    if self.high {
                        if self.slave_phase < 1.0 {
                            break;
                        }
                        self.slave_phase -= 1.0;
                        let t = self.slave_phase / slave_frequency;
                        let mut triangle_step = (slope_up + slope_down) * slave_frequency;
                        triangle_step *= triangle_amount;

                        this_sample -= (1.0 - triangle_amount) * this_blep_sample(t);
                        next_sample -= (1.0 - triangle_amount) * next_blep_sample(t);
                        this_sample += triangle_step * this_integrated_blep_sample(t);
                        next_sample += triangle_step * next_integrated_blep_sample(t);
                        self.high = false;
                    }
                }
            }

            if enable_sync && reset {
                self.slave_phase = reset_time * slave_frequency;
                self.high = false;
            }

            next_sample += compute_naive_sample(
                self.slave_phase,
                pw,
                slope_up,
                slope_down,
                triangle_amount,
                square_amount,
            );
            self.previous_pw = pw;

            if output_phase {
                let mut phasor = self.master_phase;
                if enable_sync {
                    // A trick to prevent discontinuities when the phase wraps around.
                    let w = 4.0 * (1.0 - self.master_phase) * self.master_phase;
                    this_sample *= w * (2.0 - w);

                    // Apply some asymmetry on the main phasor too.
                    let p2 = phasor * phasor;
                    phasor += (p2 * p2 - phasor) * f32::abs(pw - 0.5) * 2.0;
                }
                *out_sample = phasor + phase_modulation.next() * this_sample;
            } else {
                *out_sample = 2.0 * this_sample - 1.0;
            }
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
    square_amount: f32,
) -> f32 {
    let mut saw = phase;
    let square = if phase < pw { 0.0 } else { 1.0 };
    let triangle = if phase < pw {
        phase * slope_up
    } else {
        1.0 - (phase - pw) * slope_down
    };
    saw += (square - saw) * square_amount;
    saw += (triangle - saw) * triangle_amount;

    saw
}
