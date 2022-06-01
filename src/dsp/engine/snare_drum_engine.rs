//! 808 and synthetic snare drum generators.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::dsp::drums::analog_snare_drum::AnalogSnareDrum;
use crate::dsp::drums::synthetic_snare_drum::SyntheticSnareDrum;

#[derive(Debug, Default)]
pub struct SnareDrumEngine {
    analog_snare_drum: AnalogSnareDrum,
    synthetic_snare_drum: SyntheticSnareDrum,
}

impl SnareDrumEngine {
    fn new() -> Self {
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

        let sustain = matches!(
            parameters.trigger,
            TriggerState::Unpatched | TriggerState::UnpatchedAutotriggered
        );

        let trigger = matches!(
            parameters.trigger,
            TriggerState::RisingEdge | TriggerState::UnpatchedAutotriggered
        );

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
