//! FM Operator.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::dsp::oscillator::sine_oscillator::sine_pm;

#[derive(Debug, Default)]
pub struct Operator {
    pub phase: u32,
    pub amplitude: f32,
}

impl Operator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.phase = 0;
        self.amplitude = 0.0;
    }
}

pub enum ModulationSource {
    External = -2,
    None = -1,
    Feedback = 0,
}

pub type RenderFn = fn(
    ops: &mut [Operator],
    f: &[f32],
    a: &[f32],
    fb_state: &mut [f32],
    fb_amount: i32,
    modulation: &[f32],
    out: &mut [f32],
);

#[allow(clippy::too_many_arguments)]
pub fn render_operators<const N: usize, const MODULATION_SOURCE: i32, const ADDITIVE: bool>(
    ops: &mut [Operator],
    f: &[f32],
    a: &[f32],
    fb_state: &mut [f32],
    fb_amount: i32,
    modulation: &[f32],
    out: &mut [f32],
) {
    let mut previous_0 = 0.0;
    let mut previous_1 = 0.0;

    if MODULATION_SOURCE >= ModulationSource::Feedback as i32 {
        previous_0 = fb_state[0];
        previous_1 = fb_state[1];
    }

    let mut frequency = [0u32; N];
    let mut phase = [0u32; N];
    let mut amplitude = [0.0; N];
    let mut amplitude_increment = [0.0; N];

    let scale = 1.0 / out.len() as f32;

    for i in 0..N {
        frequency[i] = (f32::min(f[i], 0.5) * 4294967296.0) as u32;
        phase[i] = ops[i].phase;
        amplitude[i] = ops[i].amplitude;
        amplitude_increment[i] = (f32::min(a[i], 4.0) - amplitude[i]) * scale;
    }

    let fb_scale = if fb_amount != 0 {
        (1 << fb_amount) as f32 / 512.0
    } else {
        0.0
    };

    for (out_sample, modulation) in out.iter_mut().zip(modulation.iter()) {
        let mut pm = 0.0;

        if MODULATION_SOURCE >= ModulationSource::Feedback as i32 {
            pm = (previous_0 + previous_1) * fb_scale;
        } else if MODULATION_SOURCE == ModulationSource::External as i32 {
            pm = *modulation;
        }

        for i in 0..N {
            phase[i] = phase[i].wrapping_add(frequency[i]);
            pm = sine_pm(phase[i], pm) * amplitude[i];
            amplitude[i] += amplitude_increment[i];
            if i == MODULATION_SOURCE as usize {
                previous_1 = previous_0;
                previous_0 = pm;
            }
        }

        if ADDITIVE {
            *out_sample += pm;
        } else {
            *out_sample = pm;
        }
    }

    for i in 0..N {
        ops[i].phase = phase[i];
        ops[i].amplitude = amplitude[i];
    }

    if MODULATION_SOURCE >= ModulationSource::Feedback as i32 {
        fb_state[0] = previous_0;
        fb_state[1] = previous_1;
    }
}
