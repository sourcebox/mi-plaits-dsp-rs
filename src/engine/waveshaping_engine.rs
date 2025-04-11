//! Waveshaping oscillator.
//!
//! Asymmetric triangle processed by a waveshaper and a wavefolder.
//!
//! Engine parameters:
//! - *HARMONICS:* waveshaper waveform.
//! - *TIMBRE:* wavefolder amount.
//! - *MORPH:* waveform asymmetry.
//!
//! *AUX* signal: variant employing another wavefolder curve, as available in *Warps*.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use super::{note_to_frequency, Engine, EngineParameters};
use crate::oscillator::oscillator::{Oscillator, OscillatorShape};
use crate::oscillator::sine_oscillator::sine;
use crate::resources::{fold::LUT_FOLD, fold::LUT_FOLD_2, waveshape::LOOKUP_TABLE_I16_TABLE};
use crate::stmlib::dsp::interpolate_hermite;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;

#[derive(Debug, Default)]
pub struct WaveshapingEngine {
    slope: Oscillator,
    triangle: Oscillator,
    previous_shape: f32,
    previous_wavefolder_gain: f32,
    previous_overtone_gain: f32,
}

impl WaveshapingEngine {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Engine for WaveshapingEngine {
    fn init(&mut self) {
        self.slope.init();
        self.triangle.init();
        self.previous_shape = 0.0;
        self.previous_wavefolder_gain = 0.0;
        self.previous_overtone_gain = 0.0;
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        let root = parameters.note;

        let f0 = note_to_frequency(root);
        let pw = parameters.morph * 0.45 + 0.5;

        // Start from bandlimited slope signal.
        self.slope
            .render(f0, pw, None, out, OscillatorShape::Slope, false);
        self.triangle
            .render(f0, 0.5, None, aux, OscillatorShape::Slope, false);

        // Try to estimate how rich the spectrum is, and reduce the range of the
        // waveshaping control accordingly.
        let slope = 3.0 + (parameters.morph - 0.5).abs() * 5.0;
        let shape_amount = (parameters.harmonics - 0.5).abs() * 2.0;
        let shape_amount_attenuation = tame(f0, slope, 16.0);
        let wavefolder_gain = parameters.timbre;
        let wavefolder_gain_attenuation = tame(
            f0,
            slope * (3.0 + shape_amount * shape_amount_attenuation * 5.0),
            12.0,
        );

        // Apply waveshaper / wavefolder.
        let mut shape_modulation = ParameterInterpolator::new(
            &mut self.previous_shape,
            0.5 + (parameters.harmonics - 0.5) * shape_amount_attenuation,
            out.len(),
        );
        let mut wf_gain_modulation = ParameterInterpolator::new(
            &mut self.previous_wavefolder_gain,
            0.03 + 0.46 * wavefolder_gain * wavefolder_gain_attenuation,
            out.len(),
        );
        let overtone_gain = parameters.timbre * (2.0 - parameters.timbre);
        let mut overtone_gain_modulation = ParameterInterpolator::new(
            &mut self.previous_overtone_gain,
            overtone_gain * (2.0 - overtone_gain),
            out.len(),
        );

        for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter_mut()) {
            let shape = shape_modulation.next() * 3.9999;
            let shape_integral = shape as usize;
            let shape_fractional = shape - (shape_integral as f32);

            let shape_1 = LOOKUP_TABLE_I16_TABLE[shape_integral];
            let shape_2 = LOOKUP_TABLE_I16_TABLE[shape_integral + 1];

            let ws_index = 127.0 * *out_sample + 128.0;
            let mut ws_index_integral = ws_index as usize;
            let ws_index_fractional = ws_index - (ws_index_integral as f32);
            ws_index_integral &= 255;

            let x0 = (shape_1[ws_index_integral] as f32) / 32768.0;
            let x1 = (shape_1[ws_index_integral + 1] as f32) / 32768.0;
            let x = x0 + (x1 - x0) * ws_index_fractional;

            let y0 = (shape_2[ws_index_integral] as f32) / 32768.0;
            let y1 = (shape_2[ws_index_integral + 1] as f32) / 32768.0;
            let y = y0 + (y1 - y0) * ws_index_fractional;

            let mix = x + (y - x) * shape_fractional;
            let index = mix * wf_gain_modulation.next() + 0.5;
            let fold = interpolate_hermite(&LUT_FOLD[1..], index, 512.0);
            let fold_2 = -interpolate_hermite(&LUT_FOLD_2[1..], index, 512.0);

            let sine = sine(*aux_sample * 0.25 + 0.5);
            *out_sample = fold;
            *aux_sample = sine + (fold_2 - sine) * overtone_gain_modulation.next();
        }
    }
}

#[inline]
fn tame(mut f0: f32, harmonics: f32, order: f32) -> f32 {
    f0 *= harmonics;
    let max_f = 0.5 / order;
    let mut max_amount = 1.0 - (f0 - max_f) / (0.5 - max_f);
    max_amount = max_amount.clamp(0.0, 1.0);

    max_amount * max_amount * max_amount
}
