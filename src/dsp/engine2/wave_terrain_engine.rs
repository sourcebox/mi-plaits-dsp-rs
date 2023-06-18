//! Wave terrain synthesis - a 2D function evaluated along an elliptical path of
//! adjustable center and excentricity.
//!
//! Engine parameters:
//! - *HARMONICS:* terrain.
//! - *TIMBRE:* path radius.
//! - *MORPH:* path offset.
//!
//! *OUT* signal: direct terrain height (z).
//! *AUX* signal: terrain height interpreted as phase distortion (sin(y+z)).

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

use num_traits::{Float, FromPrimitive, Num, ToPrimitive};

use crate::dsp::allocate_buffer;
use crate::dsp::engine::{note_to_frequency, Engine, EngineParameters};
use crate::dsp::oscillator::sine_oscillator::{sine, FastSineOscillator};
use crate::dsp::oscillator::wavetable_oscillator::interpolate_wave;
use crate::dsp::resources::waves::WAV_INTEGRATED_WAVES;
use crate::stmlib::dsp::parameter_interpolator::SimpleParameterInterpolator;

#[derive(Debug)]
pub struct WaveTerrainEngine<'a> {
    path: FastSineOscillator,
    offset: f32,
    terrain_idx: f32,

    temp_buffer_1: &'a mut [f32],
    temp_buffer_2: &'a mut [f32],
    user_terrain: Option<&'a [i16]>,
}

impl<'a> WaveTerrainEngine<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            path: FastSineOscillator::new(),
            offset: 0.0,
            terrain_idx: 0.0,
            temp_buffer_1: allocate_buffer(buffer_allocator, block_size * 2).unwrap(),
            temp_buffer_2: allocate_buffer(buffer_allocator, block_size * 2).unwrap(),
            user_terrain: None,
        }
    }

    pub fn set_user_terrain(&mut self, user_terrain: Option<&'a [i16]>) {
        self.user_terrain = user_terrain;
    }

    #[inline]
    fn terrain(&self, x: f32, y: f32, terrain_index: usize) -> f32 {
        // The Sine function only works for a positive argument.
        // Thus, all calls to Sine include a positive offset of the argument!
        let k = 4.0;

        match terrain_index {
            0 => (squash(sine(k + x * 1.273), 2.0) - sine(k + y * (x + 1.571) * 0.637)) * 0.57,
            1 => {
                let xy = x * y;
                sine(k + sine(k + (x + y) * 0.637) / (0.2 + xy * xy) * 0.159)
            }
            2 => {
                let xy = x * y;
                sine(k + sine(k + 2.387 * xy) / (0.350 + xy * xy) * 0.159)
            }
            3 => {
                let xy = x * y;
                let xys = (x - 0.25) * (y + 0.25);
                sine(k + xy / (2.0 + f32::abs(5.0 * xys)) * 6.366)
            }
            4 => sine(
                0.159 / (0.170 + f32::abs(y - 0.25))
                    + 0.477 / (0.350 + f32::abs((x + 0.5) * (y + 1.5)))
                    + k,
            ),
            5 | 6 | 7 => terrain_lookup_wt(x, y, 2 - (terrain_index - 5) as i32),
            8 => terrain_lookup(x, y, self.user_terrain.unwrap_or_default()),
            _ => 0.0,
        }
    }
}

impl<'a> Engine for WaveTerrainEngine<'a> {
    fn init(&mut self) {
        self.path.init();
        self.offset = 0.0;
        self.terrain_idx = 0.0;
        self.user_terrain = None;
    }

    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        const OVERSAMPLING: usize = 2;
        const SCALE: f32 = 1.0 / OVERSAMPLING as f32;

        let f0 = note_to_frequency(parameters.note);
        let attenuation = f32::max(1.0 - 8.0 * f0, 0.0);
        let radius = 0.1 + 0.9 * parameters.timbre * attenuation * (2.0 - attenuation);

        // Use the "magic sine" algorithm to generate sin and cos functions for the
        // trajectory coordinates.
        self.path
            .render_quadrature(f0 * SCALE, radius, self.temp_buffer_1, self.temp_buffer_2);

        let offset =
            SimpleParameterInterpolator::new(self.offset, 1.9 * parameters.morph - 1.0, out.len());
        let num_terrains = match self.user_terrain {
            Some(_) => 9,
            None => 8,
        };
        let terrain_idx = SimpleParameterInterpolator::new(
            self.terrain_idx,
            f32::min(parameters.harmonics * 1.05, 1.0) * (num_terrains as f32 - 1.0001),
            out.len(),
        );

        let mut ij = 0;

        for (aux_sample, out_sample) in aux.iter_mut().zip(out.iter_mut()) {
            let x_offset = offset.update(&mut self.offset);

            let z = terrain_idx.update(&mut self.terrain_idx);
            let z_integral = z as usize;
            let z_fractional = z - (z_integral as f32);

            let mut out_s = 0.0;
            let mut aux_s = 0.0;

            for _ in 0..OVERSAMPLING {
                let x = self.temp_buffer_1[ij] * (1.0 - f32::abs(x_offset)) + x_offset;
                let y = self.temp_buffer_2[ij];
                ij += 1;

                let z0 = self.terrain(x, y, z_integral);
                let z1 = self.terrain(x, y, z_integral + 1);
                let z = z0 + (z1 - z0) * z_fractional;
                out_s += z;
                aux_s += y + z;
            }

            *out_sample = SCALE * out_s;
            *aux_sample = sine(1.0 + 0.5 * SCALE * aux_s);
        }
    }
}

#[inline]
fn terrain_lookup(mut x: f32, mut y: f32, terrain: &[i16]) -> f32 {
    let terrain_size = 64;
    let value_scale = 1.0 / 128.0;
    let coord_scale = (terrain_size - 2) as f32 * 0.5;

    x = (x + 1.0) * coord_scale;
    y = (y + 1.0) * coord_scale;

    let x_integral = x as usize;
    let x_fractional = x - (x_integral as f32);
    let y_integral = y as usize;
    let y_fractional = y - (y_integral as f32);

    let mut xy = [0.0; 2];

    let mut terrain_index = y_integral * terrain_size;
    xy[0] = interpolate_wave(&terrain[terrain_index..], x_integral, x_fractional);
    terrain_index += terrain_size;
    xy[1] = interpolate_wave(&terrain[terrain_index..], x_integral, x_fractional);

    (xy[0] + (xy[1] - xy[0]) * y_fractional) * value_scale
}

#[inline]
fn interpolate_integrated_wave<T>(table: &[T], index_integral: usize, index_fractional: f32) -> f32
where
    T: Num + FromPrimitive + ToPrimitive,
{
    let a = table[index_integral].to_f32().unwrap_or_default();
    let b = table[index_integral + 1].to_f32().unwrap_or_default();
    let c = table[index_integral + 2].to_f32().unwrap_or_default();
    let t = index_fractional;

    (b - a) + (c - b - b + a) * t
}

// Lookup from the wavetable data re-interpreted as a terrain. :facepalm:
#[inline]
fn terrain_lookup_wt(x: f32, y: f32, bank: i32) -> f32 {
    let table_size = 128;
    let table_size_full = table_size + 4; // Includes 4 wrapped samples
    let num_waves = 64;
    let sample = (y + 1.0) * 0.5 * table_size as f32;
    let wt = (x + 1.0) * 0.5 * (num_waves - 1) as f32;

    let waves = &WAV_INTEGRATED_WAVES;
    let mut waves_index = (bank * num_waves * table_size_full) as usize;

    let sample_integral = sample as usize;
    let sample_fractional = sample - (sample_integral as f32);
    let wt_integral = wt as usize;
    let wt_fractional = wt - (wt_integral as f32);

    let mut xy = [0.0; 2];

    let value_scale = 1.0 / 1024.0;
    waves_index += wt_integral * table_size_full as usize;
    xy[0] = interpolate_integrated_wave(&waves[waves_index..], sample_integral, sample_fractional);
    waves_index += table_size_full as usize;
    xy[1] = interpolate_integrated_wave(&waves[waves_index..], sample_integral, sample_fractional);

    (xy[0] + (xy[1] - xy[0]) * wt_fractional) * value_scale
}

#[inline]
fn squash(mut x: f32, a: f32) -> f32 {
    x *= a;

    x / (1.0 + f32::abs(x))
}
