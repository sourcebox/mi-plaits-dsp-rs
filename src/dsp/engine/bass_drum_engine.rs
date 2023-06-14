//! Analog bass drum model.
//!
//! Behavioral simulation of circuits from classic drum machines.
//!
//! Engine parameters:
//! - *HARMONICS:* attack sharpness and amount of overdrive.
//! - *TIMBRE:* brightness.
//! - *MORPH:* decay time.
//!
//! Outputs:
//! - *OUT* signal: bridged T-network excited by a nicely shaped pulse.
//! - *AUX* signal: frequency-modulated triangle VCO, turned into a sine with a pair of diodes,
//!   and shaped by a dirty VCA.
//!
//! Without any signal patched to the *TRIG* input, a continuous tone is produced.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::dsp::drums::analog_bass_drum::AnalogBassDrum;
use crate::dsp::drums::synthetic_bass_drum::SyntheticBassDrum;
use crate::dsp::fx::overdrive::Overdrive;

#[derive(Debug, Default)]
pub struct BassDrumEngine {
    analog_bass_drum: AnalogBassDrum,
    synthetic_bass_drum: SyntheticBassDrum,

    overdrive: Overdrive,
}

impl BassDrumEngine {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Engine for BassDrumEngine {
    fn init(&mut self) {
        self.analog_bass_drum.init();
        self.synthetic_bass_drum.init();
        self.overdrive.init();
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

        let attack_fm_amount = f32::min(parameters.harmonics * 4.0, 1.0);
        let self_fm_amount = f32::max(f32::min(parameters.harmonics * 4.0 - 1.0, 1.0), 0.0);
        let drive =
            f32::max(parameters.harmonics * 2.0 - 1.0, 0.0) * f32::max(1.0 - 16.0 * f0, 0.0);

        let sustain = matches!(parameters.trigger, TriggerState::Unpatched);
        let trigger = matches!(parameters.trigger, TriggerState::RisingEdge);

        self.analog_bass_drum.render(
            sustain,
            trigger,
            parameters.accent,
            f0,
            parameters.timbre,
            parameters.morph,
            attack_fm_amount,
            self_fm_amount,
            out,
        );

        self.overdrive.process(0.5 + 0.5 * drive, out);

        self.synthetic_bass_drum.render(
            sustain,
            trigger,
            parameters.accent,
            f0,
            parameters.timbre,
            parameters.morph,
            if sustain {
                parameters.harmonics
            } else {
                0.4 - 0.25 * parameters.morph * parameters.morph
            },
            f32::min(parameters.harmonics * 2.0, 1.0),
            f32::max(parameters.harmonics * 2.0 - 1.0, 0.0),
            aux,
        );
    }
}
