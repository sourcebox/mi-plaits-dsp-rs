//! Chords.
//!
//! Four-note chords, played by virtual analogue or wavetable oscillators.
//! The virtual analogue oscillators emulate the stack of harmonically-related square or
//! sawtooth waveforms generated by vintage string & organ machines.
//!
//! Engine parameters:
//! - *HARMONICS:* chord type.
//! - *TIMBRE*: chord inversion and transposition.
//! - *MORPH:* waveform. The first half of the range goes through a selection of string-machine
//!   like raw waveforms (different combinations of the organ and string “drawbars”),
//!   the second half of the knob scans a small wavetable containing 16 waveforms.
//!
//! *AUX* signal: root note of the chord.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters};
use crate::dsp::chords::chord_bank::{ChordBank, CHORD_NUM_VOICES};
use crate::dsp::oscillator::string_synth_oscillator::StringSynthOscillator;
use crate::dsp::oscillator::wavetable_oscillator::WavetableOscillator;
use crate::dsp::resources::WAV_INTEGRATED_WAVES;
use crate::stmlib::dsp::one_pole;

const CHORD_NUM_HARMONICS: usize = 3;

#[derive(Debug)]
pub struct ChordEngine<'a> {
    divide_down_voice: [StringSynthOscillator; CHORD_NUM_VOICES],
    wavetable_voice: [WavetableOscillator; CHORD_NUM_VOICES],
    chords: ChordBank,

    morph_lp: f32,
    timbre_lp: f32,

    wavetable: [&'a [i16]; 15],
}

impl<'a> ChordEngine<'a> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> Default for ChordEngine<'a> {
    fn default() -> Self {
        Self {
            divide_down_voice: [
                StringSynthOscillator::default(),
                StringSynthOscillator::default(),
                StringSynthOscillator::default(),
                StringSynthOscillator::default(),
                StringSynthOscillator::default(),
            ],
            wavetable_voice: [
                WavetableOscillator::default(),
                WavetableOscillator::default(),
                WavetableOscillator::default(),
                WavetableOscillator::default(),
                WavetableOscillator::default(),
            ],
            chords: ChordBank::new(),
            morph_lp: 0.0,
            timbre_lp: 0.0,
            wavetable: [
                &WAV_INTEGRATED_WAVES[wt_index(2, 6, 1)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 6, 6)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 6, 4)..],
                &WAV_INTEGRATED_WAVES[wt_index(0, 6, 0)..],
                &WAV_INTEGRATED_WAVES[wt_index(0, 6, 1)..],
                &WAV_INTEGRATED_WAVES[wt_index(0, 6, 2)..],
                &WAV_INTEGRATED_WAVES[wt_index(0, 6, 7)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 4, 7)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 4, 6)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 4, 5)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 4, 4)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 4, 3)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 4, 2)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 4, 1)..],
                &WAV_INTEGRATED_WAVES[wt_index(2, 4, 0)..],
            ],
        }
    }
}

impl<'a> Engine for ChordEngine<'a> {
    fn init(&mut self) {
        for i in 0..CHORD_NUM_VOICES {
            self.divide_down_voice[i].init();
            self.wavetable_voice[i].init();
        }

        self.chords.init();

        self.morph_lp = 0.0;
        self.timbre_lp = 0.0;

        self.reset();
    }

    fn reset(&mut self) {
        self.chords.reset();
    }

    #[inline]
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

        let mut harmonics: [f32; CHORD_NUM_HARMONICS * 2 + 2] = [0.0; CHORD_NUM_HARMONICS * 2 + 2];
        let mut note_amplitudes: [f32; CHORD_NUM_VOICES] = [0.0; CHORD_NUM_VOICES];
        let registration = f32::max(1.0 - self.morph_lp * 2.15, 0.0);

        compute_registration(registration, &mut harmonics);
        harmonics[CHORD_NUM_HARMONICS * 2] = 0.0;

        let mut ratios: [f32; CHORD_NUM_VOICES] = [0.0; CHORD_NUM_VOICES];
        let aux_note_mask =
            self.chords
                .compute_chord_inversion(self.timbre_lp, &mut ratios, &mut note_amplitudes);

        out.fill(0.0);
        aux.fill(0.0);

        let f0 = note_to_frequency(parameters.note) * 0.998;
        let waveform = f32::max((self.morph_lp - 0.535) * 2.15, 0.0);

        for note in 0..CHORD_NUM_VOICES {
            let mut wavetable_amount = 50.0 * (self.morph_lp - FADE_POINT[note]);
            wavetable_amount = wavetable_amount.clamp(0.0, 1.0);

            let mut divide_down_amount = 1.0 - wavetable_amount;
            let destination = ((1 << note) & aux_note_mask) != 0;

            let note_f0 = f0 * ratios[note];
            let mut divide_down_gain = 4.0 - note_f0 * 32.0;
            divide_down_gain = divide_down_gain.clamp(0.0, 1.0);
            divide_down_amount *= divide_down_gain;

            if wavetable_amount > 0.0 {
                if destination {
                    self.wavetable_voice[note].render(
                        note_f0 * 1.004,
                        note_amplitudes[note] * wavetable_amount,
                        waveform,
                        &self.wavetable,
                        aux,
                        256,
                        15,
                        true,
                    );
                } else {
                    self.wavetable_voice[note].render(
                        note_f0 * 1.004,
                        note_amplitudes[note] * wavetable_amount,
                        waveform,
                        &self.wavetable,
                        out,
                        256,
                        15,
                        true,
                    );
                }
            }

            if divide_down_amount > 0.0 {
                if destination {
                    self.divide_down_voice[note].render(
                        note_f0,
                        &harmonics,
                        note_amplitudes[note] * divide_down_amount,
                        aux,
                    );
                } else {
                    self.divide_down_voice[note].render(
                        note_f0,
                        &harmonics,
                        note_amplitudes[note] * divide_down_amount,
                        out,
                    );
                }
            }
        }

        for (out_sample, aux_sample) in out.iter_mut().zip(aux.iter_mut()) {
            *out_sample += *aux_sample;
            *aux_sample *= 3.0;
        }
    }
}

impl<'a> ChordEngine<'a> {}

const fn wt_index(bank: usize, row: usize, column: usize) -> usize {
    (bank * 64 + row * 8 + column) * 260
}

#[inline]
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

const REGISTRATION_TABLE_SIZE: usize = 8;

const REGISTRATIONS: [[f32; CHORD_NUM_HARMONICS * 2]; REGISTRATION_TABLE_SIZE] = [
    [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],    // Square
    [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],    // Saw
    [0.5, 0.0, 0.5, 0.0, 0.0, 0.0],    // Saw + saw
    [0.33, 0.0, 0.33, 0.0, 0.33, 0.0], // Full saw
    [0.33, 0.0, 0.0, 0.33, 0.0, 0.33], // Full saw + square hybrid
    [0.5, 0.0, 0.0, 0.0, 0.0, 0.5],    // Saw + high square harmo
    [0.0, 0.5, 0.0, 0.0, 0.0, 0.5],    // Square + high square harmo
    [0.0, 0.1, 0.1, 0.0, 0.2, 0.6],    // // Saw+square + high harmo
];

const FADE_POINT: [f32; CHORD_NUM_VOICES] = [0.55, 0.47, 0.49, 0.51, 0.53];
