//! Analog snare drum model.
//!
//! Engine parameters:
//! - *HARMONICS:* balance of the harmonic and noisy components.
//! - *TIMBRE:* balance between the different modes of the drum.
//! - *MORPH:* decay time.
//!
//! Outputs:
//! - *OUT* signal: bunch of bridged T-networks, one for each mode of
//!   the shell, excited by a nicely shaped pulse; plus some band-pass filtered noise.
//! - *AUX* signal: pair of frequency-modulated sine VCO, mixed with high-pass filtered noise.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::drums::analog_snare_drum::AnalogSnareDrum;
use crate::drums::synthetic_snare_drum::SyntheticSnareDrum;

#[derive(Debug, Default, Clone)]
pub struct SnareDrumEngine {
    analog_snare_drum: AnalogSnareDrum,
    synthetic_snare_drum: SyntheticSnareDrum,
}

impl SnareDrumEngine {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Engine for SnareDrumEngine {
    fn init(&mut self) {
        self.analog_snare_drum.init();
        self.synthetic_snare_drum.init();
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

        let sustain = matches!(parameters.trigger, TriggerState::Unpatched);
        let trigger = matches!(parameters.trigger, TriggerState::RisingEdge);

        self.analog_snare_drum.render(
            sustain,
            trigger,
            parameters.accent,
            f0,
            parameters.timbre,
            parameters.morph,
            parameters.harmonics,
            out,
        );

        self.synthetic_snare_drum.render(
            sustain,
            trigger,
            parameters.accent,
            f0,
            parameters.timbre,
            parameters.morph,
            parameters.harmonics,
            aux,
        );
    }
}
