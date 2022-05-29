//! Simple modal synthesis voice with a mallet exciter: click -> LPF -> resonator.
//!
//! The click is replaced by continuous white noise when the trigger input
//! of the module is not patched.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::resonator::{Resonator, ResonatorSvf, MAX_NUM_MODES};
use crate::dsp::noise::dust::dust;
use crate::stmlib::dsp::filter::FilterMode;
use crate::stmlib::dsp::units::semitones_to_ratio;

#[derive(Debug, Default)]
pub struct ModalVoice {
    excitation_filter: ResonatorSvf<1>,
    resonator: Resonator,
}

impl ModalVoice {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.excitation_filter.init();
        self.resonator.init(0.015, MAX_NUM_MODES);
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        sustain: bool,
        trigger: bool,
        accent: f32,
        f0: f32,
        structure: f32,
        mut brightness: f32,
        mut damping: f32,
        temp: &mut [f32],
        temp_2: &mut [f32],
        out: &mut [f32],
        aux: &mut [f32],
    ) {
        let density = brightness * brightness;

        brightness += 0.25 * accent * (1.0 - brightness);
        damping += 0.25 * accent * (1.0 - damping);

        let range = if sustain { 36.0 } else { 60.0 };
        let f = if sustain { 4.0 * f0 } else { 2.0 * f0 };
        let cutoff = f32::min(
            f * semitones_to_ratio((brightness * (2.0 - brightness) - 0.5) * range),
            0.499,
        );
        let q = if sustain { 0.7 } else { 1.5 };

        // Synthesize excitation signal.
        if sustain {
            let dust_f = 0.00005 + 0.99995 * density * density;
            for sample_temp in temp.iter_mut() {
                *sample_temp = dust(dust_f) * (4.0 - dust_f * 3.0) * accent;
            }
        } else {
            for temp_sample in temp.iter_mut() {
                *temp_sample = 0.0;
            }
            if trigger {
                let attenuation = 1.0 - damping * 0.5;
                let amplitude = (0.12 + 0.08 * accent) * attenuation;
                temp[0] = amplitude * semitones_to_ratio(cutoff * cutoff * 24.0) / cutoff;
            }
        }

        self.excitation_filter.process(
            core::slice::from_ref(&cutoff),
            core::slice::from_ref(&q),
            core::slice::from_ref(&1.0),
            temp,
            temp_2,
            FilterMode::LowPass,
            false,
        );

        for (aux_sample, temp_2_sample) in aux.iter_mut().zip(temp_2.iter()) {
            *aux_sample += *temp_2_sample;
        }

        self.resonator
            .process(f0, structure, brightness, damping, temp_2, out);
    }
}
