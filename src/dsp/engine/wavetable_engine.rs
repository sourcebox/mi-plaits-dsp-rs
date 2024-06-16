//! Wavetable oscillator.
//!
//! Four banks of 8x8 waveforms, accessed by row and column, with or without interpolation.
//!
//! Banks:
//! - Bank A: harmonically poor waveforms obtained by additive synthesis
//!   (sine harmonics, drawbar organ waveforms).
//! - Bank B: harmonically rich waveforms obtained by formant synthesis or waveshaping.
//! - Bank C: wavetables from the Shruthi-1 / Ambika, sampled from classic wavetable
//!   or ROM playback synths.
//! - Bank D: a joyous semi-random permutation of waveforms from the other 3 banks.
//!
//! Engine parameters:
//! - *HARMONICS:* bank selection. 4 interpolated banks followed by the same 4 banks,
//!   in reverse order, without interpolation.
//! - *TIMBRE:* row index. Within a row, the waves are sorted by spectral brightness
//!   (except for bank D which is a mess!).
//! - *MORPH:* column index.
//!
//! *AUX* signal: low-fi (5-bit) output.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters};
use crate::dsp::oscillator::wavetable_oscillator::{interpolate_wave_hermite, Differentiator};
use crate::dsp::resources::waves::WAV_INTEGRATED_WAVES;
use crate::dsp::A0;
use crate::stmlib::dsp::one_pole;
use crate::stmlib::dsp::parameter_interpolator::SimpleParameterInterpolator;

const TABLE_SIZE: usize = 128;
const TABLE_SIZE_F: f32 = TABLE_SIZE as f32;

#[derive(Debug)]
pub struct WavetableEngine<'a> {
    phase: f32,

    x_pre_lp: f32,
    y_pre_lp: f32,
    z_pre_lp: f32,

    x_lp: f32,
    y_lp: f32,
    z_lp: f32,

    previous_x: f32,
    previous_y: f32,
    previous_z: f32,
    previous_f0: f32,

    diff_out: Differentiator,

    wavetables: &'a [i16; 25344],
}

impl<'a> Default for WavetableEngine<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> WavetableEngine<'a> {
    pub fn new() -> Self {
        Self {
            phase: 0.0,

            x_pre_lp: 0.0,
            y_pre_lp: 0.0,
            z_pre_lp: 0.0,

            x_lp: 0.0,
            y_lp: 0.0,
            z_lp: 0.0,

            previous_x: 0.0,
            previous_y: 0.0,
            previous_z: 0.0,
            previous_f0: 0.0,

            diff_out: Differentiator::new(),

            wavetables: &WAV_INTEGRATED_WAVES,
        }
    }

    pub fn set_wavetables(&mut self, wavetables: &'a [i16; 25344]) {
        self.wavetables = wavetables;
    }

    #[inline]
    fn read_wave(
        &self,
        x: usize,
        y: usize,
        z: usize,
        randomize: usize,
        phase_integral: usize,
        phase_fractional: f32,
    ) -> f32 {
        let wave = ((x + y * 8 + z * 64) * randomize) % 192;
        interpolate_wave_hermite(
            &self.wavetables[wave * (TABLE_SIZE + 4)..],
            phase_integral,
            phase_fractional,
        )
    }
}

impl<'a> Engine for WavetableEngine<'a> {
    fn init(&mut self) {
        self.phase = 0.0;

        self.x_lp = 0.0;
        self.y_lp = 0.0;
        self.z_lp = 0.0;

        self.x_pre_lp = 0.0;
        self.y_pre_lp = 0.0;
        self.z_pre_lp = 0.0;

        self.previous_x = 0.0;
        self.previous_y = 0.0;
        self.previous_z = 0.0;
        self.previous_f0 = A0;

        self.diff_out.init();
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

        one_pole(&mut self.x_pre_lp, parameters.timbre * 6.9999, 0.2);
        one_pole(&mut self.y_pre_lp, parameters.morph * 6.9999, 0.2);
        one_pole(&mut self.z_pre_lp, parameters.harmonics * 6.9999, 0.05);

        let x = self.x_pre_lp;
        let y = self.y_pre_lp;
        let z = self.z_pre_lp;

        let quantization = (z - 3.0).clamp(0.0, 1.0);
        let lp_coefficient = (2.0 * f0 * (4.0 - 3.0 * quantization)).clamp(0.01, 0.1);

        let x_integral = x as usize;
        let mut x_fractional = x - (x_integral as f32);
        let y_integral = y as usize;
        let mut y_fractional = y - (y_integral as f32);
        let z_integral = z as usize;
        let mut z_fractional = z - (z_integral as f32);

        x_fractional += quantization * (clamp(x_fractional, 16.0) - x_fractional);
        y_fractional += quantization * (clamp(y_fractional, 16.0) - y_fractional);
        z_fractional += quantization * (clamp(z_fractional, 16.0) - z_fractional);

        let x_modulation = SimpleParameterInterpolator::new(
            self.previous_x,
            x_integral as f32 + x_fractional,
            out.len(),
        );
        let y_modulation = SimpleParameterInterpolator::new(
            self.previous_y,
            y_integral as f32 + y_fractional,
            out.len(),
        );
        let z_modulation = SimpleParameterInterpolator::new(
            self.previous_z,
            z_integral as f32 + z_fractional,
            out.len(),
        );

        let f0_modulation = SimpleParameterInterpolator::new(self.previous_f0, f0, out.len());

        for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter_mut()) {
            let f0 = f0_modulation.update(&mut self.previous_f0);

            let gain = (1.0 / (f0 * 131072.0)) * (0.95 - f0);
            let cutoff = f32::min(TABLE_SIZE_F * f0, 1.0);

            one_pole(
                &mut self.x_lp,
                x_modulation.update(&mut self.previous_x),
                lp_coefficient,
            );
            one_pole(
                &mut self.y_lp,
                y_modulation.update(&mut self.previous_y),
                lp_coefficient,
            );
            one_pole(
                &mut self.z_lp,
                z_modulation.update(&mut self.previous_z),
                lp_coefficient,
            );

            let x = self.x_lp;
            let y = self.y_lp;
            let z = self.z_lp;

            let x_integral = x as usize;
            let x_fractional = x - (x_integral as f32);
            let y_integral = y as usize;
            let y_fractional = y - (y_integral as f32);
            let z_integral = z as usize;
            let z_fractional = z - (z_integral as f32);

            self.phase += f0;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }

            let p = self.phase * TABLE_SIZE_F;
            let p_integral = p as usize;
            let p_fractional = p - (p_integral as f32);

            {
                let x0 = x_integral;
                let x1 = x_integral + 1;
                let y0 = y_integral;
                let y1 = y_integral + 1;
                let mut z0 = z_integral;
                let mut z1 = z_integral + 1;

                if z0 >= 4 {
                    z0 = 7 - z0;
                }
                if z1 >= 4 {
                    z1 = 7 - z1;
                }

                let r0 = if z0 == 3 { 101 } else { 1 };
                let r1 = if z1 == 3 { 101 } else { 1 };

                let x0y0z0 = self.read_wave(x0, y0, z0, r0, p_integral, p_fractional);
                let x1y0z0 = self.read_wave(x1, y0, z0, r0, p_integral, p_fractional);
                let xy0z0 = x0y0z0 + (x1y0z0 - x0y0z0) * x_fractional;

                let x0y1z0 = self.read_wave(x0, y1, z0, r0, p_integral, p_fractional);
                let x1y1z0 = self.read_wave(x1, y1, z0, r0, p_integral, p_fractional);
                let xy1z0 = x0y1z0 + (x1y1z0 - x0y1z0) * x_fractional;

                let xyz0 = xy0z0 + (xy1z0 - xy0z0) * y_fractional;

                let x0y0z1 = self.read_wave(x0, y0, z1, r1, p_integral, p_fractional);
                let x1y0z1 = self.read_wave(x1, y0, z1, r1, p_integral, p_fractional);
                let xy0z1 = x0y0z1 + (x1y0z1 - x0y0z1) * x_fractional;

                let x0y1z1 = self.read_wave(x0, y1, z1, r1, p_integral, p_fractional);
                let x1y1z1 = self.read_wave(x1, y1, z1, r1, p_integral, p_fractional);
                let xy1z1 = x0y1z1 + (x1y1z1 - x0y1z1) * x_fractional;

                let xyz1 = xy0z1 + (xy1z1 - xy0z1) * y_fractional;

                let mut mix = xyz0 + (xyz1 - xyz0) * z_fractional;
                mix = self.diff_out.process(cutoff, mix) * gain;
                *out_sample = mix;
                *aux_sample = (((mix * 32.0) as i32) as f32) / 32.0;
            }
        }
    }
}

#[inline]
fn clamp(mut x: f32, amount: f32) -> f32 {
    x -= 0.5;
    x *= amount;
    x = x.clamp(-0.5, 0.5);
    x += 0.5;

    x
}
