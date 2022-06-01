//! Additive synthesis with 24+8 partials.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use num_traits::float::Float;

use super::{note_to_frequency, Engine, EngineParameters};
use crate::dsp::oscillator::harmonic_oscillator::HarmonicOscillator;
use crate::dsp::resources::LUT_SINE;
use crate::stmlib::dsp::{interpolate_wrap, one_pole};

const HARMONIC_BATCH_SIZE: usize = 12;
const NUM_HARMONICS: usize = 36;
const NUM_HARMONIC_OSCILLATORS: usize = NUM_HARMONICS / HARMONIC_BATCH_SIZE;

#[derive(Debug)]
pub struct AdditiveEngine {
    harmonic_oscillator: [HarmonicOscillator<HARMONIC_BATCH_SIZE>; NUM_HARMONIC_OSCILLATORS],
    amplitudes: [f32; NUM_HARMONICS],
}

impl Default for AdditiveEngine {
    fn default() -> Self {
        Self {
            harmonic_oscillator: [
                HarmonicOscillator::<HARMONIC_BATCH_SIZE>::default(),
                HarmonicOscillator::<HARMONIC_BATCH_SIZE>::default(),
                HarmonicOscillator::<HARMONIC_BATCH_SIZE>::default(),
            ],
            amplitudes: [0.0; NUM_HARMONICS],
        }
    }
}

impl AdditiveEngine {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Engine for AdditiveEngine {
    fn init(&mut self) {
        self.amplitudes = [0.0; NUM_HARMONICS];
        for osc in self.harmonic_oscillator.iter_mut() {
            osc.init();
        }
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        let f0 = note_to_frequency(parameters.note);

        let centroid = parameters.timbre;
        let raw_bumps = parameters.harmonics;
        let raw_slope = (1.0 - 0.6 * raw_bumps) * parameters.morph;
        let slope = 0.01 + 1.99 * raw_slope * raw_slope * raw_slope;
        let bumps = 16.0 * raw_bumps * raw_bumps;
        update_amplitudes(
            centroid,
            slope,
            bumps,
            &mut self.amplitudes[..],
            &INTEGER_HARMONICS,
        );
        self.harmonic_oscillator[0].render(f0, &self.amplitudes[..12], out, 1);
        self.harmonic_oscillator[1].render(f0, &self.amplitudes[12..], out, 13);

        update_amplitudes(
            centroid,
            slope,
            bumps,
            &mut self.amplitudes[24..],
            &ORGAN_HARMONICS,
        );

        self.harmonic_oscillator[2].render(f0, &self.amplitudes[24..], aux, 1);
    }
}

const INTEGER_HARMONICS: [usize; 24] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
];

const ORGAN_HARMONICS: [usize; 8] = [0, 1, 2, 3, 5, 7, 9, 11];

#[inline]
fn update_amplitudes(
    centroid: f32,
    slope: f32,
    bumps: f32,
    amplitudes: &mut [f32],
    harmonic_indices: &[usize],
) {
    let n = (harmonic_indices.len() as f32) - 1.0;
    let margin = (1.0 / slope - 1.0) / (1.0 + bumps);
    let center = centroid * (n + margin) - 0.5 * margin;

    let mut sum = 0.001;

    for (i, _) in harmonic_indices.iter().enumerate() {
        let order = ((i as f32) - center).abs() * slope;
        let mut gain = 1.0 - order;
        gain += gain.abs();
        gain *= gain;

        let b = 0.25 + order * bumps;
        let bump_factor = 1.0 + interpolate_wrap(&LUT_SINE, b, 1024.0);

        gain *= bump_factor;
        gain *= gain;
        gain *= gain;

        let j = harmonic_indices[i];

        // Warning about the following line: this is not a proper LP filter
        // because of the normalization. But in spite of its strange working,
        // this line turns out ot be absolutely essential.
        //
        // I have tried both normalizing the LP-ed spectrum, and LP-ing the
        // normalized spectrum, and both of them cause more annoyances than this
        // "incorrect" solution.

        one_pole(&mut amplitudes[j], gain, 0.001);
        sum += amplitudes[j];
    }

    sum = 1.0 / sum;

    for i in 0..harmonic_indices.len() {
        amplitudes[harmonic_indices[i]] *= sum;
    }
}
