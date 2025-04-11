//! Naive speech synth - made from "synthesizer" building blocks (pulse
//! oscillator and zero-delay SVF).

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::oscillator::oscillator::{Oscillator, OscillatorShape};
use crate::stmlib::dsp::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::stmlib::dsp::units::semitones_to_ratio;
use crate::{A0, SAMPLE_RATE};

const NUM_FORMANTS: usize = 5;
const NUM_PHONEMES: usize = 5;
const NUM_REGISTERS: usize = 5;

#[derive(Debug, Default)]
pub struct NaiveSpeechSynth {
    pulse: Oscillator,
    frequency: f32,
    click_duration: usize,

    filter: [Svf; NUM_FORMANTS],
    pulse_coloration: Svf,
}

impl NaiveSpeechSynth {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.pulse.init();
        self.frequency = 0.0;
        self.click_duration = 0;

        for filter in &mut self.filter {
            filter.init();
        }
        self.pulse_coloration.init();
        self.pulse_coloration
            .set_f_q(800.0 / SAMPLE_RATE, 0.5, FrequencyApproximation::Dirty);
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        click: bool,
        mut frequency: f32,
        phoneme: f32,
        vocal_register: f32,
        temp: &mut [f32],
        excitation: &mut [f32],
        output: &mut [f32],
    ) {
        if click {
            self.click_duration = (SAMPLE_RATE * 0.05) as usize;
        }
        self.click_duration -= usize::min(self.click_duration, output.len());

        if self.click_duration != 0 {
            frequency *= 0.5;
        }

        // Generate excitation signal (glottal pulse).
        self.pulse.render(
            frequency,
            0.5,
            None,
            excitation,
            OscillatorShape::ImpulseTrain,
            false,
        );
        self.pulse_coloration
            .process_buffer(excitation, temp, FilterMode::BandPass);
        excitation.copy_from_slice(temp);
        for excitation_sample in excitation.iter_mut() {
            *excitation_sample *= 4.0;
        }

        let p = phoneme * (NUM_PHONEMES as f32 - 1.001);
        let r = vocal_register * (NUM_REGISTERS as f32 - 1.001);

        let p_integral = p as usize;
        let p_fractional = p - (p_integral as f32);
        let r_integral = r as usize;
        let r_fractional = r - (r_integral as f32);

        output.fill(0.0);

        for i in 0..NUM_FORMANTS {
            let p0r0 = PHONEMES[p_integral][r_integral].formant[i];
            let p0r1 = PHONEMES[p_integral][r_integral + 1].formant[i];
            let p1r0 = PHONEMES[p_integral + 1][r_integral].formant[i];
            let p1r1 = PHONEMES[p_integral + 1][r_integral + 1].formant[i];

            let p0r_f = (p0r0
                .frequency
                .wrapping_add(p0r1.frequency.wrapping_sub(p0r0.frequency)))
                as f32
                * r_fractional;
            let p1r_f = (p1r0
                .frequency
                .wrapping_add(p1r1.frequency.wrapping_sub(p1r0.frequency)))
                as f32
                * r_fractional;
            let mut f = p0r_f + (p1r_f - p0r_f) * p_fractional;

            let p0r_a = (p0r0
                .amplitude
                .wrapping_add(p0r1.amplitude.wrapping_sub(p0r0.amplitude)))
                as f32
                * r_fractional;
            let p1r_a = (p1r0
                .amplitude
                .wrapping_add(p1r1.amplitude.wrapping_sub(p1r0.amplitude)))
                as f32
                * r_fractional;
            let a = (p0r_a + (p1r_a - p0r_a) * p_fractional) / 256.0;

            if f >= 160.0 {
                f = 160.0;
            }
            f = A0 * semitones_to_ratio(f - 33.0);
            if self.click_duration != 0 && i == 0 {
                f *= 0.5;
            }

            self.filter[i].set_f_q(f, 20.0, FrequencyApproximation::Dirty);
            self.filter[i].process_add_buffer(excitation, output, a, FilterMode::BandPass);
        }
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

struct Phoneme {
    formant: [Formant; NUM_FORMANTS],
}

impl Phoneme {
    pub const fn new(formant: [Formant; NUM_FORMANTS]) -> Self {
        Self { formant }
    }
}

const PHONEMES: [[Phoneme; NUM_PHONEMES]; NUM_REGISTERS] = [
    [
        Phoneme::new([
            Formant::new(74, 255),
            Formant::new(83, 114),
            Formant::new(97, 90),
            Formant::new(98, 90),
            Formant::new(100, 25),
        ]),
        Phoneme::new([
            Formant::new(75, 255),
            Formant::new(84, 128),
            Formant::new(100, 114),
            Formant::new(101, 101),
            Formant::new(103, 20),
        ]),
        Phoneme::new([
            Formant::new(76, 255),
            Formant::new(85, 128),
            Formant::new(100, 18),
            Formant::new(102, 16),
            Formant::new(104, 3),
        ]),
        Phoneme::new([
            Formant::new(79, 255),
            Formant::new(85, 161),
            Formant::new(101, 25),
            Formant::new(104, 4),
            Formant::new(110, 0),
        ]),
        Phoneme::new([
            Formant::new(79, 255),
            Formant::new(85, 128),
            Formant::new(101, 6),
            Formant::new(106, 25),
            Formant::new(110, 0),
        ]),
    ],
    [
        Phoneme::new([
            Formant::new(67, 255),
            Formant::new(91, 64),
            Formant::new(98, 90),
            Formant::new(101, 64),
            Formant::new(102, 32),
        ]),
        Phoneme::new([
            Formant::new(67, 255),
            Formant::new(92, 51),
            Formant::new(99, 64),
            Formant::new(103, 51),
            Formant::new(105, 25),
        ]),
        Phoneme::new([
            Formant::new(69, 255),
            Formant::new(93, 51),
            Formant::new(100, 32),
            Formant::new(102, 25),
            Formant::new(103, 25),
        ]),
        Phoneme::new([
            Formant::new(67, 255),
            Formant::new(91, 16),
            Formant::new(100, 8),
            Formant::new(103, 4),
            Formant::new(110, 0),
        ]),
        Phoneme::new([
            Formant::new(65, 255),
            Formant::new(95, 25),
            Formant::new(101, 45),
            Formant::new(105, 2),
            Formant::new(110, 0),
        ]),
    ],
    [
        Phoneme::new([
            Formant::new(59, 255),
            Formant::new(92, 8),
            Formant::new(99, 40),
            Formant::new(102, 20),
            Formant::new(104, 10),
        ]),
        Phoneme::new([
            Formant::new(61, 255),
            Formant::new(94, 45),
            Formant::new(101, 32),
            Formant::new(103, 25),
            Formant::new(105, 8),
        ]),
        Phoneme::new([
            Formant::new(60, 255),
            Formant::new(93, 16),
            Formant::new(101, 16),
            Formant::new(104, 4),
            Formant::new(105, 4),
        ]),
        Phoneme::new([
            Formant::new(65, 255),
            Formant::new(92, 25),
            Formant::new(100, 8),
            Formant::new(105, 4),
            Formant::new(110, 0),
        ]),
        Phoneme::new([
            Formant::new(60, 255),
            Formant::new(96, 64),
            Formant::new(101, 12),
            Formant::new(106, 12),
            Formant::new(110, 1),
        ]),
    ],
    [
        Phoneme::new([
            Formant::new(67, 255),
            Formant::new(78, 72),
            Formant::new(98, 22),
            Formant::new(99, 25),
            Formant::new(101, 2),
        ]),
        Phoneme::new([
            Formant::new(67, 255),
            Formant::new(79, 80),
            Formant::new(99, 64),
            Formant::new(101, 64),
            Formant::new(102, 12),
        ]),
        Phoneme::new([
            Formant::new(68, 255),
            Formant::new(79, 80),
            Formant::new(100, 12),
            Formant::new(102, 20),
            Formant::new(103, 5),
        ]),
        Phoneme::new([
            Formant::new(69, 255),
            Formant::new(79, 90),
            Formant::new(101, 40),
            Formant::new(104, 10),
            Formant::new(110, 0),
        ]),
        Phoneme::new([
            Formant::new(69, 255),
            Formant::new(79, 72),
            Formant::new(101, 20),
            Formant::new(106, 20),
            Formant::new(110, 0),
        ]),
    ],
    [
        Phoneme::new([
            Formant::new(65, 255),
            Formant::new(74, 25),
            Formant::new(98, 6),
            Formant::new(100, 10),
            Formant::new(101, 4),
        ]),
        Phoneme::new([
            Formant::new(65, 255),
            Formant::new(74, 25),
            Formant::new(100, 36),
            Formant::new(101, 51),
            Formant::new(103, 12),
        ]),
        Phoneme::new([
            Formant::new(66, 255),
            Formant::new(75, 25),
            Formant::new(100, 18),
            Formant::new(102, 8),
            Formant::new(104, 5),
        ]),
        Phoneme::new([
            Formant::new(63, 255),
            Formant::new(77, 64),
            Formant::new(99, 8),
            Formant::new(104, 2),
            Formant::new(110, 0),
        ]),
        Phoneme::new([
            Formant::new(63, 255),
            Formant::new(77, 40),
            Formant::new(100, 4),
            Formant::new(106, 2),
            Formant::new(110, 0),
        ]),
    ],
];
