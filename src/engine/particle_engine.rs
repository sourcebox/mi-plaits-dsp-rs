//! Particle noise.
//!
//! Dust noise processed by networks of all-pass or band-pass filters.
//!
//! Engine parameters:
//! - *HARMONICS:* amount of frequency randomization.
//! - *TIMBRE:* particle density.
//! - *MORPH:* filter type - reverberating all-pass network before 12 o’clock,
//!   then increasingly resonant band-pass filters.
//!
//! *AUX* signal: raw dust noise.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use alloc::boxed::Box;
use alloc::vec;

#[allow(unused_imports)]
use num_traits::float::Float;

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::fx::diffuser::Diffuser;
use crate::noise::particle::Particle;
use crate::utils::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::utils::units::semitones_to_ratio;

const NUM_PARTICLES: usize = 6;

#[derive(Debug)]
pub struct ParticleEngine {
    particle: [Particle; NUM_PARTICLES],
    diffuser: Diffuser,
    post_filter: Svf,

    temp_buffer: Box<[f32]>,
}

impl ParticleEngine {
    pub fn new(block_size: usize) -> Self {
        Self {
            particle: core::array::from_fn(|_| Particle::new()),
            diffuser: Diffuser::new(),
            post_filter: Svf::new(),
            temp_buffer: vec![0.0; block_size].into_boxed_slice(),
        }
    }
}

impl Engine for ParticleEngine {
    fn init(&mut self) {
        for particle in &mut self.particle {
            particle.init();
        }
        self.diffuser.init();
        self.post_filter.init();
        self.reset();
    }

    fn reset(&mut self) {
        self.diffuser.reset();
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
        let density_sqrt = note_to_frequency(60.0 + parameters.timbre * parameters.timbre * 72.0);
        let density = density_sqrt * density_sqrt * (1.0 / NUM_PARTICLES as f32);
        let gain = 1.0 / density;
        let q_sqrt = semitones_to_ratio(if parameters.morph >= 0.5 {
            (parameters.morph - 0.5) * 120.0
        } else {
            0.0
        });
        let q = 0.5 + q_sqrt * q_sqrt;
        let spread = 48.0 * parameters.harmonics * parameters.harmonics;
        let raw_diffusion_sqrt = 2.0 * (parameters.morph - 0.5).abs();
        let raw_diffusion = raw_diffusion_sqrt * raw_diffusion_sqrt;
        let diffusion = if parameters.morph < 0.5 {
            raw_diffusion
        } else {
            0.0
        };
        let sync = matches!(parameters.trigger, TriggerState::RisingEdge);

        out.fill(0.0);
        aux.fill(0.0);

        for particle in &mut self.particle {
            particle.render(sync, density, gain, f0, spread, q, out, aux);
        }

        self.post_filter
            .set_f_q(f32::min(f0, 0.49), 0.5, FrequencyApproximation::Dirty);
        self.post_filter
            .process_buffer(out, &mut self.temp_buffer, FilterMode::LowPass);

        out.copy_from_slice(&self.temp_buffer);

        self.diffuser
            .process(0.8 * diffusion * diffusion, 0.5 * diffusion + 0.25, out);
    }
}
