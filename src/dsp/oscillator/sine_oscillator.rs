//! Simple sine oscillator (wavetable) + fast sine oscillator (magic circle).
//
//! The fast implementation might glitch a bit under heavy modulations of the
//! frequency.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::dsp::resources::LUT_SINE;
use crate::stmlib::dsp::interpolate;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::rsqrt::fast_rsqrt_carmack;

#[derive(Debug, Default)]
pub struct SineOscillator {
    // Oscillator state.
    phase: f32,

    // For interpolation of parameters.
    frequency: f32,
    amplitude: f32,
}

impl SineOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.frequency = 0.0;
        self.amplitude = 0.0;
    }

    #[inline]
    pub fn next(&mut self, mut frequency: f32) -> f32 {
        if frequency >= 0.5 {
            frequency = 0.5;
        }

        self.phase += frequency;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        interpolate(&LUT_SINE, self.phase, 1024.0)
    }

    #[inline]
    pub fn next_sin_cos(
        &mut self,
        mut frequency: f32,
        amplitude: f32,
        sin: &mut f32,
        cos: &mut f32,
    ) {
        if frequency >= 0.5 {
            frequency = 0.5;
        }

        self.phase += frequency;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        *sin = amplitude * interpolate(&LUT_SINE, self.phase, 1024.0);
        *cos = amplitude * interpolate(&LUT_SINE[256..], self.phase, 1024.0);
    }

    #[inline]
    pub fn render_add(&mut self, frequency: f32, amplitude: f32, out: &mut [f32]) {
        self.render_internal(frequency, amplitude, out, true);
    }

    #[inline]
    pub fn render(&mut self, frequency: f32, out: &mut [f32]) {
        self.render_internal(frequency, 1.0, out, false);
    }

    #[inline]
    pub fn render_internal(
        &mut self,
        mut frequency: f32,
        amplitude: f32,
        out: &mut [f32],
        additive: bool,
    ) {
        if frequency >= 0.5 {
            frequency = 0.5;
        }

        let mut fm = ParameterInterpolator::new(&mut self.frequency, frequency, out.len());
        let mut am = ParameterInterpolator::new(&mut self.amplitude, amplitude, out.len());

        for out_sample in out.iter_mut() {
            self.phase += fm.next();

            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }

            let s = interpolate(&LUT_SINE, self.phase, 1024.0);

            if additive {
                *out_sample += am.next() * s;
            } else {
                *out_sample = s;
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct FastSineOscillator {
    // Oscillator state.
    x: f32,
    y: f32,

    // For interpolation of parameters.
    epsilon: f32,
    amplitude: f32,
}

impl FastSineOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.x = 1.0;
        self.y = 0.0;
        self.epsilon = 0.0;
        self.amplitude = 0.0;
    }

    #[inline]
    pub fn render(&mut self, frequency: f32, out: &mut [f32]) {
        self.render_internal(frequency, 1.0, out, false);
    }

    #[inline]
    pub fn render_add(&mut self, frequency: f32, amplitude: f32, out: &mut [f32]) {
        self.render_internal(frequency, amplitude, out, true);
    }

    #[inline]
    pub fn render_internal(
        &mut self,
        mut frequency: f32,
        mut amplitude: f32,
        out: &mut [f32],
        additive: bool,
    ) {
        if frequency >= 0.25 {
            frequency = 0.25;
            amplitude = 0.0;
        } else {
            amplitude *= 1.0 - frequency * 4.0;
        }

        let mut epsilon =
            ParameterInterpolator::new(&mut self.epsilon, fast_2_sin(frequency), out.len());
        let mut am = ParameterInterpolator::new(&mut self.amplitude, amplitude, out.len());
        let mut x = self.x;
        let mut y = self.y;

        let norm = x * x + y * y;

        if norm <= 0.5 || norm >= 2.0 {
            let scale = fast_rsqrt_carmack(norm);
            x *= scale;
            y *= scale;
        }

        for sample in out.iter_mut() {
            let e = epsilon.next();
            x += e * y;
            y -= e * x;
            if additive {
                *sample += am.next() * x;
            } else {
                *sample = x;
            }
        }

        self.x = x;
        self.y = y;
    }
}

#[inline]
fn fast_2_sin(f: f32) -> f32 {
    // In theory, epsilon = 2 sin(pi f)
    // Here, to avoid the call to sinf, we use a 3rd order polynomial
    // approximation, which looks like a Taylor expansion, but with a
    // correction term to give a good trade-off between average error
    // (1.13 cents) and maximum error (7.33 cents) when generating sinewaves
    // in the 16 Hz to 16kHz range (with sr = 48kHz).
    let f_pi = f * core::f32::consts::PI;
    f_pi * (2.0 - (2.0 * 0.96 / 6.0) * f_pi * f_pi)
}
