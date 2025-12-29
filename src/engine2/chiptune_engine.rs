//! Chiptune waveforms with arpeggiator.
//!
//! Arpeggiator is clocked via the trigger signal.
//!
//! Engine parameters:
//! - *HARMONICS:* chord.
//! - *TIMBRE:* arpeggio type or chord inversion.
//! - *MORPH:* PW/Sync.
//! - *TIMBRE modulation:* envelope shape.
//!
//! *OUT* signal: square wave voices.
//! *AUX* signal: NES triangle voice.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use super::arpeggiator::{Arpeggiator, ArpeggiatorMode};
use crate::chords::chord_bank::{ChordBank, CHORD_NUM_VOICES};
use crate::engine::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::oscillator::nes_triangle_oscillator::NesTriangleOscillator;
use crate::oscillator::super_square_oscillator::SuperSquareOscillator;
use crate::utils::hysteresis_quantizer::HysteresisQuantizer2;
use crate::utils::one_pole;
use crate::utils::units::semitones_to_ratio;

pub const NO_ENVELOPE: f32 = 2.0;

#[derive(Debug, Default, Clone)]
pub struct ChiptuneEngine {
    voice: [SuperSquareOscillator; CHORD_NUM_VOICES],
    bass: NesTriangleOscillator,

    chords: ChordBank,
    arpeggiator: Arpeggiator,
    arpeggiator_pattern_selector: HysteresisQuantizer2,

    envelope_shape: f32,
    envelope_state: f32,
    aux_envelope_amount: f32,
    sample_rate: f32,
}

impl ChiptuneEngine {
    pub fn new() -> Self {
        Self {
            voice: core::array::from_fn(|_| SuperSquareOscillator::new()),
            bass: NesTriangleOscillator::new(),

            chords: ChordBank::new(),
            arpeggiator: Arpeggiator::new(),
            arpeggiator_pattern_selector: HysteresisQuantizer2::new(),

            envelope_shape: 0.0,
            envelope_state: 0.0,
            aux_envelope_amount: 0.0,
            sample_rate: 48000.0,
        }
    }

    #[inline]
    pub fn set_envelope_shape(&mut self, envelope_shape: f32) {
        self.envelope_shape = envelope_shape;
    }
}

impl Engine for ChiptuneEngine {
    fn init(&mut self, sample_rate_hz: f32) {
        self.sample_rate = sample_rate_hz;
        self.bass.init();

        for voice in &mut self.voice {
            voice.init();
        }

        self.chords.init();

        self.arpeggiator.init();

        self.arpeggiator_pattern_selector.init(12, 0.075, false);

        self.envelope_shape = NO_ENVELOPE;
        self.envelope_state = 0.0;
        self.aux_envelope_amount = 0.0;
    }

    fn reset(&mut self) {
        self.chords.reset();
    }

    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        already_enveloped: &mut bool,
    ) {
        let f0 = note_to_frequency(parameters.note, parameters.a0_normalized);
        let shape = parameters.morph * 0.995;
        let clocked = parameters.trigger != TriggerState::Unpatched;
        let mut root_transposition = 1.0;

        *already_enveloped = clocked;

        if clocked {
            if parameters.trigger == TriggerState::RisingEdge {
                self.chords.set_chord(parameters.harmonics);
                self.chords.sort();

                let pattern = self.arpeggiator_pattern_selector.process(parameters.timbre);
                self.arpeggiator
                    .set_mode(ArpeggiatorMode::from(pattern / 3));
                self.arpeggiator.set_range(1 << (pattern % 3));
                self.arpeggiator.clock(self.chords.num_notes());
                self.envelope_state = 1.0;
            }

            let octave = (1 << self.arpeggiator.octave()) as f32;
            let note_f0 = f0 * self.chords.sorted_ratio(self.arpeggiator.note()) * octave;
            root_transposition = octave;
            self.voice[0].render(note_f0, shape, out);
        } else {
            let mut ratios = [0.0; CHORD_NUM_VOICES];
            let mut amplitudes = [0.0; CHORD_NUM_VOICES];

            self.chords.set_chord(parameters.harmonics);
            self.chords
                .compute_chord_inversion(parameters.timbre, &mut ratios, &mut amplitudes);

            for j in (1..CHORD_NUM_VOICES).step_by(2) {
                amplitudes[j] = -amplitudes[j];
            }

            out.fill(0.0);

            for (n, voice) in self.voice.iter_mut().enumerate() {
                let voice_f0 = f0 * ratios[n];
                voice.render(voice_f0, shape, aux);
                for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter_mut()) {
                    *out_sample += *aux_sample * amplitudes[n];
                }
            }
        }

        // Render bass note.
        self.bass.render(f0 * 0.5 * root_transposition, aux, 5);

        // Apply envelope if necessary.
        if self.envelope_shape != NO_ENVELOPE {
            let shape = f32::abs(self.envelope_shape);
            let decay = 1.0 - 2.0 / self.sample_rate * semitones_to_ratio(60.0 * shape) * shape;
            let aux_envelope_amount = (self.envelope_shape * 20.0).clamp(0.0, 1.0);

            for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter_mut()) {
                one_pole(&mut self.aux_envelope_amount, aux_envelope_amount, 0.01);
                self.envelope_state *= decay;
                *out_sample *= self.envelope_state;
                *aux_sample *= 1.0 + self.aux_envelope_amount * (self.envelope_state - 1.0);
            }
        }
    }
}
