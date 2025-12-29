//! SAM-inspired speech synth (as used in Shruthi/Ambika/Braids).

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::oscillator::sine_oscillator::sine_raw;
use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::polyblep::{next_blep_sample, this_blep_sample};

const NUM_FORMANTS: usize = 3;
const NUM_VOWELS: usize = 9;
const NUM_CONSONANTS: usize = 8;
const NUM_PHONEMES: usize = NUM_VOWELS + NUM_CONSONANTS;

#[derive(Debug, Default, Clone)]
pub struct SamSpeechSynth {
    phase: f32,
    frequency: f32,

    pulse_next_sample: f32,
    pulse_lp: f32,

    formant_phase: [u32; 3],
    consonant_samples: usize,
    consonant_index: f32,

    sample_rate_hz: f32,
}

impl SamSpeechSynth {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        self.sample_rate_hz = sample_rate_hz;
        self.phase = 0.0;
        self.frequency = 0.0;
        self.pulse_next_sample = 0.0;
        self.pulse_lp = 0.0;

        self.formant_phase.fill(0);
        self.consonant_samples = 0;
        self.consonant_index = 0.0;
    }

    #[inline]
    pub fn render(
        &mut self,
        consonant: bool,
        mut frequency: f32,
        vowel: f32,
        formant_shift: f32,
        excitation: &mut [f32],
        output: &mut [f32],
    ) {
        if frequency >= 0.0625 {
            frequency = 0.0625;
        }

        if consonant {
            self.consonant_samples = (self.sample_rate_hz * 0.05) as usize;
            let r = (vowel + 3.0 * frequency + 7.0 * formant_shift) * 8.0;
            self.consonant_index = (r as usize % NUM_CONSONANTS) as f32;
        }
        self.consonant_samples -= usize::min(self.consonant_samples, output.len());

        let phoneme = if self.consonant_samples != 0 {
            self.consonant_index + NUM_VOWELS as f32
        } else {
            vowel * (NUM_VOWELS as f32 - 1.0001)
        };

        let mut formant_frequency: [u32; NUM_FORMANTS] = [0; NUM_FORMANTS];
        let mut formant_amplitude: [f32; NUM_FORMANTS] = [0.0; NUM_FORMANTS];

        interpolate_phoneme_data(
            phoneme,
            formant_shift,
            formant_frequency.as_mut_slice(),
            formant_amplitude.as_mut_slice(),
            self.sample_rate_hz,
        );

        let mut fm = ParameterInterpolator::new(&mut self.frequency, frequency, output.len());
        let mut pulse_next_sample = self.pulse_next_sample;

        for (output_sample, excitation_sample) in output.iter_mut().zip(excitation.iter_mut()) {
            let mut pulse_this_sample = pulse_next_sample;
            pulse_next_sample = 0.0;
            let frequency = fm.next();
            self.phase += frequency;

            if self.phase >= 1.0 {
                self.phase -= 1.0;
                let t = self.phase / frequency;
                self.formant_phase[0] = (t * (formant_frequency[0]) as f32) as u32;
                self.formant_phase[1] = (t * (formant_frequency[1]) as f32) as u32;
                self.formant_phase[2] = (t * (formant_frequency[2]) as f32) as u32;
                pulse_this_sample -= this_blep_sample(t);
                pulse_next_sample -= next_blep_sample(t);
            } else {
                self.formant_phase[0] = self.formant_phase[0].wrapping_add(formant_frequency[0]);
                self.formant_phase[1] = self.formant_phase[1].wrapping_add(formant_frequency[1]);
                self.formant_phase[2] = self.formant_phase[2].wrapping_add(formant_frequency[2]);
            }
            pulse_next_sample += self.phase;

            let d = pulse_this_sample - 0.5 - self.pulse_lp;
            self.pulse_lp += f32::min(16.0 * frequency, 1.0) * d;
            *excitation_sample = d;

            let mut s = 0.0;
            s += sine_raw(self.formant_phase[0]) * formant_amplitude[0];
            s += sine_raw(self.formant_phase[1]) * formant_amplitude[1];
            s += sine_raw(self.formant_phase[2]) * formant_amplitude[2];
            s *= 1.0 - self.phase;
            *output_sample = s;
        }

        self.pulse_next_sample = pulse_next_sample;
    }
}

fn interpolate_phoneme_data(
    phoneme: f32,
    mut formant_shift: f32,
    formant_frequency: &mut [u32],
    formant_amplitude: &mut [f32],
    sample_rate_hz: f32,
) {
    let phoneme_integral = phoneme as usize;
    let phoneme_fractional = phoneme - (phoneme_integral as f32);

    let p_1 = PHONEMES[phoneme_integral];
    let p_2 = PHONEMES[phoneme_integral + 1];

    formant_shift = 1.0 + formant_shift * 2.5;

    for i in 0..NUM_FORMANTS {
        let f_1 = p_1.formant[i].frequency;
        let f_2 = p_2.formant[i].frequency;
        let mut f = (f_1.wrapping_add(f_2.wrapping_sub(f_1))) as f32 * phoneme_fractional;
        f *= 8.0 * formant_shift * 4294967296.0 / sample_rate_hz;
        formant_frequency[i] = f as u32;

        let a_1 = FORMANT_AMPLITUDE_LUT[p_1.formant[i].amplitude as usize];
        let a_2 = FORMANT_AMPLITUDE_LUT[p_2.formant[i].amplitude as usize];
        formant_amplitude[i] = a_1 + (a_2 - a_1) * phoneme_fractional;
    }
}

#[derive(Clone, Copy)]
struct Formant {
    frequency: u8,
    amplitude: u8,
}

impl Formant {
    pub const fn new(frequency: u8, amplitude: u8) -> Self {
        Self {
            frequency,
            amplitude,
        }
    }
}

#[derive(Clone, Copy)]
struct Phoneme {
    formant: [Formant; NUM_FORMANTS],
}

impl Phoneme {
    pub const fn new(formant: [Formant; NUM_FORMANTS]) -> Self {
        Self { formant }
    }
}

const PHONEMES: [Phoneme; NUM_PHONEMES + 1] = [
    Phoneme::new([
        Formant::new(60, 15),
        Formant::new(90, 13),
        Formant::new(200, 1),
    ]),
    Phoneme::new([
        Formant::new(40, 13),
        Formant::new(114, 12),
        Formant::new(139, 6),
    ]),
    Phoneme::new([
        Formant::new(33, 14),
        Formant::new(155, 12),
        Formant::new(209, 7),
    ]),
    Phoneme::new([
        Formant::new(22, 13),
        Formant::new(189, 10),
        Formant::new(247, 8),
    ]),
    Phoneme::new([
        Formant::new(51, 15),
        Formant::new(99, 12),
        Formant::new(195, 1),
    ]),
    Phoneme::new([
        Formant::new(29, 13),
        Formant::new(65, 8),
        Formant::new(180, 0),
    ]),
    Phoneme::new([
        Formant::new(13, 12),
        Formant::new(103, 3),
        Formant::new(182, 0),
    ]),
    Phoneme::new([
        Formant::new(20, 15),
        Formant::new(114, 3),
        Formant::new(213, 0),
    ]),
    Phoneme::new([
        Formant::new(13, 7),
        Formant::new(164, 3),
        Formant::new(222, 14),
    ]),
    Phoneme::new([
        Formant::new(13, 9),
        Formant::new(121, 9),
        Formant::new(254, 0),
    ]),
    Phoneme::new([
        Formant::new(40, 12),
        Formant::new(112, 10),
        Formant::new(114, 5),
    ]),
    Phoneme::new([
        Formant::new(24, 13),
        Formant::new(54, 8),
        Formant::new(157, 0),
    ]),
    Phoneme::new([
        Formant::new(33, 14),
        Formant::new(155, 12),
        Formant::new(166, 7),
    ]),
    Phoneme::new([
        Formant::new(36, 14),
        Formant::new(83, 8),
        Formant::new(249, 1),
    ]),
    Phoneme::new([
        Formant::new(40, 14),
        Formant::new(114, 12),
        Formant::new(139, 6),
    ]),
    Phoneme::new([
        Formant::new(13, 5),
        Formant::new(58, 5),
        Formant::new(182, 5),
    ]),
    Phoneme::new([
        Formant::new(13, 7),
        Formant::new(164, 10),
        Formant::new(222, 14),
    ]),
    // GUARD
    Phoneme::new([
        Formant::new(13, 7),
        Formant::new(164, 10),
        Formant::new(222, 14),
    ]),
];

#[allow(clippy::excessive_precision)]
const FORMANT_AMPLITUDE_LUT: [f32; 16] = [
    0.03125000, 0.03756299, 0.04515131, 0.05427259, 0.06523652, 0.07841532, 0.09425646, 0.11329776,
    0.13618570, 0.16369736, 0.19676682, 0.23651683, 0.28429697, 0.34172946, 0.41076422, 0.49374509,
];
