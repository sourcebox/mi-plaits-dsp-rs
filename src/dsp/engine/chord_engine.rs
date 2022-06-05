//! Chords: wavetable and divide-down organ/string machine.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters};
use crate::dsp::oscillator::string_synth_oscillator::StringSynthOscillator;
use crate::dsp::oscillator::wavetable_oscillator::WavetableOscillator;
use crate::dsp::resources::WAV_INTEGRATED_WAVES;
use crate::stmlib::dsp::hysteresis_quantizer::HysteresisQuantizer;
use crate::stmlib::dsp::one_pole;
use crate::stmlib::dsp::units::semitones_to_ratio;

const CHORD_NUM_NOTES: usize = 4;
const CHORD_NUM_VOICES: usize = 5;
const CHORD_NUM_HARMONICS: usize = 3;
const CHORD_NUM_CHORDS: usize = 11;

#[derive(Debug)]
pub struct ChordEngine<'a> {
    divide_down_voice: [StringSynthOscillator; CHORD_NUM_VOICES],
    wavetable_voice: [WavetableOscillator; CHORD_NUM_VOICES],
    chord_index_quantizer: HysteresisQuantizer,

    morph_lp: f32,
    timbre_lp: f32,

    ratios: [f32; CHORD_NUM_CHORDS * CHORD_NUM_NOTES],

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
            chord_index_quantizer: HysteresisQuantizer::default(),
            morph_lp: 0.0,
            timbre_lp: 0.0,
            ratios: [0.0; CHORD_NUM_CHORDS * CHORD_NUM_NOTES],
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

        self.chord_index_quantizer.init();

        self.morph_lp = 0.0;
        self.timbre_lp = 0.0;

        self.reset();
    }

    fn reset(&mut self) {
        for (i, _) in CHORDS.iter().enumerate().take(CHORD_NUM_CHORDS) {
            for j in 0..CHORD_NUM_NOTES {
                self.ratios[i * CHORD_NUM_NOTES + j] = semitones_to_ratio(CHORDS[i][j]);
            }
        }
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

        let chord_index = self
            .chord_index_quantizer
            .process_with_default(parameters.harmonics * 1.02, CHORD_NUM_CHORDS);

        let mut harmonics: [f32; CHORD_NUM_HARMONICS * 2 + 2] = [0.0; CHORD_NUM_HARMONICS * 2 + 2];
        let mut note_amplitudes: [f32; CHORD_NUM_VOICES] = [0.0; CHORD_NUM_VOICES];
        let registration = f32::max(1.0 - self.morph_lp * 2.15, 0.0);

        compute_registration(registration, &mut harmonics);
        harmonics[CHORD_NUM_HARMONICS * 2] = 0.0;

        let mut ratios: [f32; CHORD_NUM_VOICES] = [0.0; CHORD_NUM_VOICES];
        let aux_note_mask = self.compute_chord_inversion(
            chord_index,
            self.timbre_lp,
            &mut ratios,
            &mut note_amplitudes,
        );

        out.fill(0.0);
        aux.fill(0.0);

        let f0 = note_to_frequency(parameters.note) * 0.998;
        let waveform = f32::max((self.morph_lp - 0.535) * 2.15, 0.0);

        for note in 0..CHORD_NUM_VOICES {
            let mut wavetable_amount = 50.0 * (self.morph_lp - FADE_POINT[note]);
            wavetable_amount = wavetable_amount.clamp(0.0, 1.0);

            let mut divide_down_amount = 1.0 - wavetable_amount;
            let destination = ((1 << note) & aux_note_mask) != 0;

            let note_f0 = f0 * ratios[note] as f32;
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

impl<'a> ChordEngine<'a> {
    #[inline]
    fn compute_chord_inversion(
        &self,
        chord_index: i32,
        mut inversion: f32,
        ratios: &mut [f32],
        amplitudes: &mut [f32],
    ) -> i32 {
        inversion *= (CHORD_NUM_NOTES * 5) as f32;

        let inversion_integral = inversion as usize;
        let inversion_fractional = inversion - (inversion_integral as f32);

        let num_rotations = inversion_integral / CHORD_NUM_NOTES;
        let rotated_note = inversion_integral % CHORD_NUM_NOTES;

        const BASE_GAIN: f32 = 0.25;

        let mut mask = 0;

        for i in 0..CHORD_NUM_NOTES {
            let transposition = 0.25
                * (1 << ((CHORD_NUM_NOTES - 1 + inversion_integral - i) / CHORD_NUM_NOTES)) as f32;
            let target_voice =
                (i.wrapping_sub(num_rotations).wrapping_add(CHORD_NUM_VOICES)) % CHORD_NUM_VOICES;
            let previous_voice =
                (target_voice.wrapping_sub(1).wrapping_add(CHORD_NUM_VOICES)) % CHORD_NUM_VOICES;

            let ratio = self.ratios[chord_index as usize * CHORD_NUM_NOTES + i];

            #[allow(clippy::comparison_chain)]
            if i == rotated_note {
                ratios[target_voice] = ratio * transposition;
                ratios[previous_voice] = ratios[target_voice] * 2.0;
                amplitudes[previous_voice] = BASE_GAIN * inversion_fractional;
                amplitudes[target_voice] = BASE_GAIN * (1.0 - inversion_fractional);
            } else if i < rotated_note {
                ratios[previous_voice] = ratio * transposition;
                amplitudes[previous_voice] = BASE_GAIN;
            } else {
                ratios[target_voice] = ratio * transposition;
                amplitudes[target_voice] = BASE_GAIN;
            }

            if i == 0 {
                if i >= rotated_note {
                    mask |= 1 << target_voice;
                }
                if i <= rotated_note {
                    mask |= 1 << previous_voice;
                }
            }
        }

        mask
    }
}

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

const CHORDS: [[f32; CHORD_NUM_NOTES]; CHORD_NUM_CHORDS] = [
    [0.00, 0.01, 11.99, 12.00], // OCT
    [0.00, 7.01, 7.00, 12.00],  // 5
    [0.00, 5.00, 7.00, 12.00],  // sus4
    [0.00, 3.00, 7.00, 12.00],  // m
    [0.00, 3.00, 7.00, 10.00],  // m7
    [0.00, 3.00, 10.00, 14.00], // m9
    [0.00, 3.00, 10.00, 17.00], // m11
    [0.00, 2.00, 9.00, 16.00],  // 69
    [0.00, 4.00, 11.00, 14.00], // M9
    [0.00, 4.00, 7.00, 11.00],  // M7
    [0.00, 4.00, 7.00, 12.00],  // M
];

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
