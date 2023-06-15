//! Integrated wavetable synthesis.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::dsp::oscillator::oscillator::MAX_FREQUENCY;
use crate::stmlib::dsp::one_pole;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;

#[derive(Debug, Default)]
pub struct WavetableOscillator {
    // Oscillator state.
    phase: f32,

    // For interpolation of parameters.
    frequency: f32,
    amplitude: f32,
    waveform: f32,
    lp: f32,

    differentiator: Differentiator,
}

impl WavetableOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.frequency = 0.0;
        self.amplitude = 0.0;
        self.waveform = 0.0;
        self.lp = 0.0;
        self.differentiator.init();
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        mut frequency: f32,
        mut amplitude: f32,
        waveform: f32,
        wavetable: &[&[i16]],
        out: &mut [f32],
        wavetable_size: usize,
        num_waves: usize,
        approximate_scale: bool,
    ) {
        if frequency >= MAX_FREQUENCY {
            frequency = MAX_FREQUENCY;
        }
        amplitude *= 1.0 - 2.0 * frequency;
        if approximate_scale {
            amplitude *= 1.0 / (frequency * 131072.0) * (0.95 - frequency);
        }

        let mut frequency_modulation =
            ParameterInterpolator::new(&mut self.frequency, frequency, out.len());
        let mut amplitude_modulation =
            ParameterInterpolator::new(&mut self.amplitude, amplitude, out.len());
        let mut waveform_modulation = ParameterInterpolator::new(
            &mut self.waveform,
            waveform * (num_waves as f32 - 1.0001),
            out.len(),
        );

        let mut lp = self.lp;
        let mut phase = self.phase;

        for out_sample in out.iter_mut() {
            let f0 = frequency_modulation.next();
            let cutoff = f32::min(wavetable_size as f32 * f0, 1.0);

            let scale = if approximate_scale {
                1.0
            } else {
                1.0 / (f0 * 131072.0) * (0.95 - f0)
            };

            phase += f0;
            if phase >= 1.0 {
                phase -= 1.0;
            }

            let waveform = waveform_modulation.next();
            let waveform_integral = waveform as usize;
            let waveform_fractional = waveform - (waveform_integral as f32);

            let p = phase * wavetable_size as f32;
            let p_integral = p as usize;
            let p_fractional = p - (p_integral as f32);

            let x0 = interpolate_wave(wavetable[waveform_integral], p_integral, p_fractional);
            let x1 = interpolate_wave(wavetable[waveform_integral + 1], p_integral, p_fractional);

            let s = self
                .differentiator
                .process(cutoff, x0 + (x1 - x0) * waveform_fractional);
            one_pole(&mut lp, s * scale, cutoff * 0.5);
            *out_sample += amplitude_modulation.next() * lp;
        }
        self.lp = lp;
        self.phase = phase;
    }
}

#[derive(Debug, Default)]
pub struct Differentiator {
    lp: f32,
    previous: f32,
}

impl Differentiator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.previous = 0.0;
        self.lp = 0.0;
    }

    #[inline]
    pub fn process(&mut self, coefficient: f32, s: f32) -> f32 {
        one_pole(&mut self.lp, s - self.previous, coefficient);
        self.previous = s;
        self.lp
    }
}

#[inline]
pub fn interpolate_wave(table: &[i16], index_integral: usize, index_fractional: f32) -> f32 {
    let a = table[index_integral] as f32;
    let b = table[index_integral + 1] as f32;
    let t = index_fractional;

    a + (b - a) * t
}

#[inline]
pub fn interpolate_wave_hermite(
    table: &[i16],
    index_integral: usize,
    index_fractional: f32,
) -> f32 {
    let xm1 = table[index_integral] as f32;
    let x0 = table[index_integral + 1] as f32;
    let x1 = table[index_integral + 2] as f32;
    let x2 = table[index_integral + 3] as f32;
    let c = (x1 - xm1) * 0.5;
    let v = x0 - x1;
    let w = c + v;
    let a = w + v + (x2 - x0) * 0.5;
    let b_neg = w + a;
    let f = index_fractional;

    (((a * f) - b_neg) * f + c) * f + x0
}
