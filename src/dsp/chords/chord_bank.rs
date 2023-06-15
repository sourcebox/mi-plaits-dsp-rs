//! Chord bank shared by several engines.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::stmlib::dsp::hysteresis_quantizer::HysteresisQuantizer2;
use crate::stmlib::dsp::units::semitones_to_ratio;

pub const CHORD_NUM_NOTES: usize = 4;
pub const CHORD_NUM_VOICES: usize = 5;
pub const CHORD_NUM_CHORDS: usize = 11;

#[derive(Debug)]
pub struct ChordBank {
    chord_index_quantizer: HysteresisQuantizer2,
    ratios: [f32; CHORD_NUM_CHORDS * CHORD_NUM_NOTES],
    note_count: [i32; CHORD_NUM_CHORDS],
    sorted_ratios: [f32; CHORD_NUM_NOTES],
}

impl Default for ChordBank {
    fn default() -> Self {
        Self {
            chord_index_quantizer: HysteresisQuantizer2::default(),
            ratios: [0.0; CHORD_NUM_CHORDS * CHORD_NUM_NOTES],
            note_count: [0; CHORD_NUM_CHORDS],
            sorted_ratios: [0.0; CHORD_NUM_NOTES],
        }
    }
}

impl ChordBank {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.chord_index_quantizer
            .init(CHORD_NUM_CHORDS as i32, 0.075, false);
        self.reset();
    }

    pub fn reset(&mut self) {
        for (i, _) in CHORDS.iter().enumerate().take(CHORD_NUM_CHORDS) {
            let mut count = 0;

            for j in 0..CHORD_NUM_NOTES {
                self.ratios[i * CHORD_NUM_NOTES + j] = semitones_to_ratio(CHORDS[i][j]);
                if CHORDS[i][j] != 0.01
                    && CHORDS[i][j] != 7.01
                    && CHORDS[i][j] != 11.99
                    && CHORDS[i][j] != 12.00
                {
                    count += 1;
                }
            }

            self.note_count[i] = count;
        }

        self.sort();
    }

    pub fn compute_chord_inversion(
        &self,
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

            let ratio = self.ratio(i as i32);

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

    #[inline]
    pub fn sort(&mut self) {
        for i in 0..CHORD_NUM_NOTES {
            let mut r = self.ratio(i as i32);
            while r > 2.0 {
                r *= 0.5;
            }
            self.sorted_ratios[i] = r;
        }
        self.sorted_ratios
            .sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    }

    #[inline]
    pub fn set_chord(&mut self, parameter: f32) {
        self.chord_index_quantizer.process(parameter * 1.02);
    }

    #[inline]
    pub fn chord_index(&self) -> i32 {
        self.chord_index_quantizer.quantized_value()
    }

    #[inline]
    fn ratio(&self, note: i32) -> f32 {
        self.ratios[self.chord_index() as usize * CHORD_NUM_NOTES + note as usize]
    }

    #[inline]
    pub fn sorted_ratio(&self, note: i32) -> f32 {
        self.sorted_ratios[note as usize]
    }

    #[inline]
    pub fn num_notes(&self) -> i32 {
        self.note_count[self.chord_index() as usize]
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
