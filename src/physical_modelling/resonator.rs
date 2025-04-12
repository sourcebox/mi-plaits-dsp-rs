//! Resonator, taken from Rings' code but with fixed position.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::resources::stiffness::LUT_STIFFNESS;
use crate::utils::cosine_oscillator::{CosineOscillator, CosineOscillatorMode};
use crate::utils::filter::{FilterMode, FrequencyApproximation, OnePole};
use crate::utils::interpolate;
use crate::utils::units::semitones_to_ratio;

pub const MAX_NUM_MODES: usize = 24;
pub const MODE_BATCH_SIZE: usize = 4;

const MODE_FILTERS_LENGTH: usize = MAX_NUM_MODES / MODE_BATCH_SIZE;

#[derive(Debug, Default)]
pub struct Resonator {
    resolution: usize,
    mode_amplitude: [f32; MAX_NUM_MODES],
    mode_filters: [ResonatorSvf<MODE_BATCH_SIZE>; MODE_FILTERS_LENGTH],
}

impl Resonator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, position: f32, resolution: usize) {
        self.resolution = usize::min(resolution, MAX_NUM_MODES);

        let mut amplitudes = CosineOscillator::new();
        amplitudes.init(position, CosineOscillatorMode::Approximate);

        for i in 0..resolution {
            self.mode_amplitude[i] = amplitudes.next() * 0.25;
        }

        for i in 0..(MAX_NUM_MODES / MODE_BATCH_SIZE) {
            self.mode_filters[i].init();
        }
    }

    #[inline]
    pub fn process(
        &mut self,
        mut f0: f32,
        structure: f32,
        mut brightness: f32,
        damping: f32,
        in_: &[f32],
        out: &mut [f32],
    ) {
        let mut stiffness = interpolate(&LUT_STIFFNESS, structure, 64.0);
        f0 *= nth_harmonic_compensation(3, stiffness);

        let mut harmonic = f0;
        let mut stretch_factor = 1.0;
        let q_sqrt = semitones_to_ratio(damping * 79.7);
        let mut q = 500.0 * q_sqrt * q_sqrt;
        brightness *= 1.0 - structure * 0.3;
        brightness *= 1.0 - damping * 0.3;
        let q_loss = brightness * (2.0 - brightness) * 0.85 + 0.15;

        let mut mode_q: [f32; MODE_BATCH_SIZE] = [0.0; MODE_BATCH_SIZE];
        let mut mode_f: [f32; MODE_BATCH_SIZE] = [0.0; MODE_BATCH_SIZE];
        let mut mode_a: [f32; MODE_BATCH_SIZE] = [0.0; MODE_BATCH_SIZE];
        let mut batch_counter = 0;

        let mut batch_processor_index = 0;
        let mut batch_processor = &mut self.mode_filters[0];

        for i in 0..self.resolution {
            let mut mode_frequency = harmonic * stretch_factor;
            if mode_frequency >= 0.499 {
                mode_frequency = 0.499;
            }
            let mode_attenuation = 1.0 - mode_frequency * 2.0;

            mode_f[batch_counter] = mode_frequency;
            mode_q[batch_counter] = 1.0 + mode_frequency * q;
            mode_a[batch_counter] = self.mode_amplitude[i] * mode_attenuation;
            batch_counter += 1;

            if batch_counter == MODE_BATCH_SIZE {
                batch_counter = 0;
                batch_processor.process(
                    &mode_f,
                    &mode_q,
                    &mode_a,
                    in_,
                    out,
                    FilterMode::BandPass,
                    true,
                );
                batch_processor_index += 1;
                if batch_processor_index < MAX_NUM_MODES / MODE_BATCH_SIZE {
                    batch_processor = &mut self.mode_filters[batch_processor_index];
                }
            }

            stretch_factor += stiffness;

            if stiffness < 0.0 {
                // Make sure that the partials do not fold back into negative
                // frequencies.
                stiffness *= 0.93;
            } else {
                // This helps adding a few extra partials in the highest
                // frequencies.
                stiffness *= 0.98;
            }

            harmonic += f0;
            q *= q_loss;
        }
    }
}

#[derive(Debug)]
pub struct ResonatorSvf<const BATCH_SIZE: usize> {
    state_1: [f32; BATCH_SIZE],
    state_2: [f32; BATCH_SIZE],
}

impl<const BATCH_SIZE: usize> Default for ResonatorSvf<BATCH_SIZE> {
    fn default() -> Self {
        Self {
            state_1: [0.0; BATCH_SIZE],
            state_2: [0.0; BATCH_SIZE],
        }
    }
}

impl<const BATCH_SIZE: usize> ResonatorSvf<BATCH_SIZE> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        for elem in self.state_1.iter_mut() {
            *elem = 0.0;
        }
        for elem in self.state_2.iter_mut() {
            *elem = 0.0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn process(
        &mut self,
        f: &[f32],
        q: &[f32],
        gain: &[f32],
        in_: &[f32],
        out: &mut [f32],
        mode: FilterMode,
        add: bool,
    ) {
        let mut g: [f32; BATCH_SIZE] = [0.0; BATCH_SIZE];
        let mut r: [f32; BATCH_SIZE] = [0.0; BATCH_SIZE];
        let mut r_plus_g: [f32; BATCH_SIZE] = [0.0; BATCH_SIZE];
        let mut h: [f32; BATCH_SIZE] = [0.0; BATCH_SIZE];
        let mut state_1: [f32; BATCH_SIZE] = [0.0; BATCH_SIZE];
        let mut state_2: [f32; BATCH_SIZE] = [0.0; BATCH_SIZE];
        let mut gains: [f32; BATCH_SIZE] = [0.0; BATCH_SIZE];

        for i in 0..BATCH_SIZE {
            g[i] = OnePole::tan(f[i], FrequencyApproximation::Fast);
            r[i] = 1.0 / q[i];
            h[i] = 1.0 / (1.0 + r[i] * g[i] + g[i] * g[i]);
            r_plus_g[i] = r[i] + g[i];
            state_1[i] = self.state_1[i];
            state_2[i] = self.state_2[i];
            gains[i] = gain[i];
        }

        for (in_sample, out_sample) in in_.iter().zip(out.iter_mut()) {
            let s_in = *in_sample;
            let mut s_out = 0.0;

            for i in 0..BATCH_SIZE {
                let hp = (s_in - r_plus_g[i] * state_1[i] - state_2[i]) * h[i];
                let bp = g[i] * hp + state_1[i];
                state_1[i] = g[i] * hp + bp;
                let lp = g[i] * bp + state_2[i];
                state_2[i] = g[i] * bp + lp;
                s_out += gains[i]
                    * (if matches!(mode, FilterMode::LowPass) {
                        lp
                    } else {
                        bp
                    });
            }

            if add {
                *out_sample += s_out;
            } else {
                *out_sample = s_out;
            }

            self.state_1[..BATCH_SIZE].copy_from_slice(&state_1[..BATCH_SIZE]);
            self.state_2[..BATCH_SIZE].copy_from_slice(&state_2[..BATCH_SIZE])
        }
    }
}

#[inline]
fn nth_harmonic_compensation(n: usize, mut stiffness: f32) -> f32 {
    let mut stretch_factor = 1.0;

    for _ in 0..(n - 1) {
        stretch_factor += stiffness;
        if stiffness < 0.0 {
            stiffness *= 0.93;
        } else {
            stiffness *= 0.98;
        }
    }

    1.0 / stretch_factor
}
