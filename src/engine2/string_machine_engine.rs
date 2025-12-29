//! String machine emulation with filter and chorus.
//!
//! Engine parameters:
//! - *HARMONICS:* chord.
//! - *TIMBRE:* chorus/filter amount.
//! - *MORPH:* waveform.
//!
//! *OUT* signal: voices 1&3 predominantly.
//! *AUX* signal: voices 2&4 predominantly.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use crate::chords::chord_bank::{ChordBank, CHORD_NUM_NOTES};
use crate::engine::chord_engine::CHORD_NUM_HARMONICS;
use crate::engine::{note_to_frequency, Engine, EngineParameters};
use crate::fx::ensemble::Ensemble;
use crate::oscillator::string_synth_oscillator::StringSynthOscillator;
use crate::utils::filter::{FilterMode, FrequencyApproximation, NaiveSvf};
use crate::utils::one_pole;
use crate::utils::units::semitones_to_ratio;

#[derive(Debug, Default, Clone)]
pub struct StringMachineEngine {
    chords: ChordBank,

    ensemble: Ensemble,
    divide_down_voice: [StringSynthOscillator; CHORD_NUM_NOTES],
    svf: [NaiveSvf; 2],

    morph_lp: f32,
    timbre_lp: f32,
}

impl StringMachineEngine {
    pub fn new() -> Self {
        Self {
            chords: ChordBank::new(),

            ensemble: Ensemble::new(),
            divide_down_voice: core::array::from_fn(|_| StringSynthOscillator::new()),
            svf: [NaiveSvf::new(), NaiveSvf::new()],

            morph_lp: 0.0,
            timbre_lp: 0.0,
        }
    }
}

impl Engine for StringMachineEngine {
    fn init(&mut self, _sample_rate_hz: f32) {
        for divide_down_voice in self.divide_down_voice.iter_mut() {
            divide_down_voice.init();
        }

        self.chords.init();
        self.morph_lp = 0.0;
        self.timbre_lp = 0.0;
        self.svf[0].init();
        self.svf[1].init();
        self.ensemble.init();
    }

    fn reset(&mut self) {
        self.chords.reset();
        self.ensemble.reset();
    }

    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        one_pole(&mut self.morph_lp, parameters.morph, 0.1);
        one_pole(&mut self.timbre_lp, parameters.timbre, 0.1);

        self.chords.set_chord(parameters.harmonics);

        let mut harmonics = [0.0; CHORD_NUM_HARMONICS * 2 + 2];
        let registration = f32::max(self.morph_lp, 0.0);
        compute_registration(registration, &mut harmonics);
        harmonics[CHORD_NUM_HARMONICS * 2] = 0.0;

        // Render string/organ sound.
        out.fill(0.0);
        aux.fill(0.0);

        let f0 = note_to_frequency(parameters.note, parameters.a0_normalized) * 0.998;
        for note in 0..CHORD_NUM_NOTES {
            let note_f0 = f0 * self.chords.ratio(note as i32);
            let divide_down_gain = (4.0 - note_f0 * 32.0).clamp(0.0, 1.0);
            self.divide_down_voice[note].render(
                note_f0,
                &harmonics,
                0.25 * divide_down_gain,
                if note & 1 != 0 { aux } else { out },
            );
        }

        // Pass through VCF.
        let cutoff = 2.2 * f0 * semitones_to_ratio(120.0 * parameters.timbre);
        self.svf[0].set_f_q(cutoff, 1.0, FrequencyApproximation::Dirty);
        self.svf[1].set_f_q(cutoff * 1.5, 1.0, FrequencyApproximation::Dirty);

        // Mixdown.
        for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter_mut()) {
            let l = self.svf[0].process(*out_sample, FilterMode::LowPass);
            let r = self.svf[1].process(*aux_sample, FilterMode::LowPass);
            *out_sample = 0.66 * l + 0.33 * r;
            *aux_sample = 0.66 * r + 0.33 * l;
        }

        // Ensemble FX.
        let amount = f32::abs(parameters.timbre - 0.5) * 2.0;
        let depth = 0.35 + 0.65 * parameters.timbre;
        self.ensemble.set_amount(amount);
        self.ensemble.set_depth(depth);
        self.ensemble.process(out, aux);
    }
}

fn compute_registration(mut registration: f32, amplitudes: &mut [f32]) {
    registration *= REGISTRATION_TABLE_SIZE as f32 - 1.001;
    let registration_integral = registration as usize;
    let registration_fractional = registration - (registration_integral as f32);

    for (i, amplitude) in amplitudes
        .iter_mut()
        .enumerate()
        .take(CHORD_NUM_HARMONICS * 2)
    {
        let a = REGISTRATIONS[registration_integral][i];
        let b = REGISTRATIONS[registration_integral + 1][i];
        *amplitude = a + (b - a) * registration_fractional;
    }
}

const REGISTRATION_TABLE_SIZE: usize = 11;

const REGISTRATIONS: [[f32; CHORD_NUM_HARMONICS * 2]; REGISTRATION_TABLE_SIZE] = [
    [1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Saw
    [0.5, 0.0, 0.5, 0.0, 0.0, 0.0], // Saw + saw
    [0.4, 0.0, 0.2, 0.0, 0.4, 0.0], // Full saw
    [0.3, 0.0, 0.0, 0.3, 0.0, 0.4], // Full saw + square hybrid
    [0.3, 0.0, 0.0, 0.0, 0.0, 0.7], // Saw + high square harmo
    [0.2, 0.0, 0.0, 0.2, 0.0, 0.6], // Weird hybrid
    [0.0, 0.2, 0.1, 0.0, 0.2, 0.5], // Sawsquare high harmo
    [0.0, 0.3, 0.0, 0.3, 0.0, 0.4], // Square high armo
    [0.0, 0.4, 0.0, 0.3, 0.0, 0.3], // Full square
    [0.0, 0.5, 0.0, 0.5, 0.0, 0.0], // Square + Square
    [0.0, 1.0, 0.0, 0.0, 0.0, 0.0], // Square
];
