//! Filtered random pulses.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters};

#[derive(Debug, Default)]
pub struct ParticleEngine {
    // TODO: implement
}

impl ParticleEngine {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Engine for ParticleEngine {
    fn init(&mut self) {
        // TODO: implement
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        // TODO: implement
    }
}
