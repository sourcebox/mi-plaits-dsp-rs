//! FM Operator.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::cell::RefCell;

use crate::oscillator::sine_oscillator::sine_pm;

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
    modulation: &RefCell<&mut [f32]>,
    out: &RefCell<&mut [f32]>,
);

#[allow(clippy::too_many_arguments)]
pub fn render_operators<const N: usize, const MODULATION_SOURCE: i32, const ADDITIVE: bool>(
    ops: &mut [Operator],
    f: &[f32],
    a: &[f32],
    fb_state: &mut [f32],
    fb_amount: i32,
    modulation: &RefCell<&mut [f32]>,
    out: &RefCell<&mut [f32]>,
) {
    let mut frequency = [0u32; N];
    let mut phase = [0u32; N];
    let mut amplitude = [0.0; N];
    let mut amplitude_increment = [0.0; N];

    let scale = 1.0 / out.borrow().len() as f32;

    for i in 0..N {
        frequency[i] = (f32::min(f[i], 0.5) * 4294967296.0) as u32;
        phase[i] = ops[i].phase;
        amplitude[i] = ops[i].amplitude;
        amplitude_increment[i] = (f32::min(a[i], 4.0) - amplitude[i]) * scale;
    }

    if MODULATION_SOURCE >= ModulationSource::Feedback as i32 {
        let fb_scale = if fb_amount != 0 {
            (1 << fb_amount) as f32 / 512.0
        } else {
            0.0
        };

        let mut previous_0 = fb_state[0];
        let mut previous_1 = fb_state[1];

        for out_sample in out.borrow_mut().iter_mut() {
            let mut pm = (previous_0 + previous_1) * fb_scale;

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

            for i in 0..N {
                ops[i].phase = phase[i];
                ops[i].amplitude = amplitude[i];
            }

            fb_state[0] = previous_0;
            fb_state[1] = previous_1;
        }
    } else if MODULATION_SOURCE == ModulationSource::External as i32 {
        let size = out.borrow().len();

        for i in 0..size {
            let mut pm = modulation.borrow()[i];

            for i in 0..N {
                phase[i] = phase[i].wrapping_add(frequency[i]);
                pm = sine_pm(phase[i], pm) * amplitude[i];
                amplitude[i] += amplitude_increment[i];
            }

            if ADDITIVE {
                out.borrow_mut()[i] += pm;
            } else {
                out.borrow_mut()[i] = pm;
            }

            for i in 0..N {
                ops[i].phase = phase[i];
                ops[i].amplitude = amplitude[i];
            }
        }
    } else {
        for out_sample in out.borrow_mut().iter_mut() {
            let mut pm = 0.0;

            for i in 0..N {
                phase[i] = phase[i].wrapping_add(frequency[i]);
                pm = sine_pm(phase[i], pm) * amplitude[i];
                amplitude[i] += amplitude_increment[i];
            }

            if ADDITIVE {
                *out_sample += pm;
            } else {
                *out_sample = pm;
            }

            for i in 0..N {
                ops[i].phase = phase[i];
                ops[i].amplitude = amplitude[i];
            }
        }
    }
}
