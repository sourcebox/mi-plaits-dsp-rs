//! Simple sine oscillator (wavetable) + fast sine oscillator (magic circle).
//
//! The fast implementation might glitch a bit under heavy modulations of the
//! frequency.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::resources::sine::{LUT_SINE, LUT_SINE_BITS, LUT_SINE_SIZE};
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::rsqrt::fast_rsqrt_carmack;
use crate::stmlib::dsp::{interpolate, interpolate_wrap};

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

        sine_no_wrap(self.phase)
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

        *sin = amplitude * sine_no_wrap(self.phase);
        *cos = amplitude * sine_no_wrap(self.phase + 0.25);
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

            let s = sine_no_wrap(self.phase);

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
        self.render_internal(frequency, 1.0, out, RenderMode::Normal);
    }

    #[inline]
    pub fn render_add(&mut self, frequency: f32, amplitude: f32, out: &mut [f32]) {
        self.render_internal(frequency, amplitude, out, RenderMode::Additive);
    }

    #[inline]
    pub fn render_quadrature(
        &mut self,
        mut frequency: f32,
        mut amplitude: f32,
        out: &mut [f32],
        out_2: &mut [f32],
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

        for (out_sample, out_2_sample) in out.iter_mut().zip(out_2.iter_mut()) {
            let e = epsilon.next();
            x += e * y;
            y -= e * x;

            let amplitude = am.next();
            *out_sample = x * amplitude;
            *out_2_sample = y * amplitude;
        }

        self.x = x;
        self.y = y;
    }

    #[inline]
    fn render_internal(
        &mut self,
        mut frequency: f32,
        mut amplitude: f32,
        out: &mut [f32],
        mode: RenderMode,
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

        for out_sample in out.iter_mut() {
            let e = epsilon.next();
            x += e * y;
            y -= e * x;

            match mode {
                RenderMode::Normal => *out_sample = x,
                RenderMode::Additive => *out_sample += am.next() * x,
            };
        }

        self.x = x;
        self.y = y;
    }
}

#[derive(Debug, Default)]
pub enum RenderMode {
    #[default]
    Normal,

    Additive,
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

// Safe for phase >= 0.0f, will wrap.
pub fn sine(phase: f32) -> f32 {
    interpolate_wrap(&LUT_SINE, phase, LUT_SINE_SIZE)
}

// Potentially unsafe, if phase >= 1.25.
pub fn sine_no_wrap(phase: f32) -> f32 {
    interpolate(&LUT_SINE, phase, LUT_SINE_SIZE)
}

// With positive of negative phase modulation up to an index of 32.
#[inline]
pub fn sine_pm(mut phase: u32, pm: f32) -> f32 {
    let max_uint32 = 4294967296.0;
    let max_index = 32;
    let offset = max_index as f32;
    let scale = max_uint32 / (max_index * 2) as f32;

    phase = phase.wrapping_add((((pm + offset) * scale) as u32).wrapping_mul(max_index * 2));
    let integral = phase >> (32 - LUT_SINE_BITS);
    let fractional = (phase << LUT_SINE_BITS) as f32 / max_uint32;
    let a = LUT_SINE[integral as usize];
    let b = LUT_SINE[integral as usize + 1];

    a + (b - a) * fractional
}

// Direct lookup without interpolation.
#[inline]
pub fn sine_raw(phase: u32) -> f32 {
    LUT_SINE[(phase >> (32 - LUT_SINE_BITS)) as usize]
}
