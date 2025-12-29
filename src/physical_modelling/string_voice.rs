//! Extended Karplus-Strong, with all the niceties from Rings.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::string::String;
use crate::noise::dust::dust;
use crate::utils::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::utils::random;
use crate::utils::units::semitones_to_ratio;

#[derive(Debug, Clone)]
pub struct StringVoice {
    excitation_filter: Svf,
    string: String,
    remaining_noise_samples: usize,
}

impl Default for StringVoice {
    fn default() -> Self {
        Self::new()
    }
}

impl StringVoice {
    pub fn new() -> Self {
        Self {
            excitation_filter: Svf::default(),
            string: String::new(),
            remaining_noise_samples: 0,
        }
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        self.excitation_filter.init();
        self.string.init(sample_rate_hz);
        self.remaining_noise_samples = 0;
    }

    pub fn reset(&mut self) {
        self.string.reset();
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

        // Synthesize excitation signal.
        if trigger || sustain {
            let range = 72.0;
            let f = 4.0 * f0;
            let cutoff = f32::min(
                f * semitones_to_ratio((brightness * (2.0 - brightness) - 0.5) * range),
                0.499,
            );
            let q = if sustain { 1.0 } else { 0.5 };
            self.remaining_noise_samples = (1.0 / f0) as usize;
            self.excitation_filter
                .set_f_q(cutoff, q, FrequencyApproximation::Dirty);
        }

        if sustain {
            let dust_f = 0.00005 + 0.99995 * density * density;

            for sample_temp in temp.iter_mut() {
                *sample_temp = dust(dust_f) * (8.0 - dust_f * 6.0) * accent;
            }
        } else if self.remaining_noise_samples > 0 {
            let mut noise_samples = usize::min(self.remaining_noise_samples, out.len());
            self.remaining_noise_samples -= noise_samples;
            let mut tail = out.len() - noise_samples;
            let mut start_index = 0;
            while noise_samples > 0 {
                temp[start_index] = 2.0 * random::get_float() - 1.0;
                start_index += 1;
                noise_samples -= 1;
            }
            while tail > 0 {
                temp[start_index] = 0.0;
                start_index += 1;
                tail -= 1;
            }
        } else {
            for sample_temp in temp.iter_mut() {
                *sample_temp = 0.0;
            }
        }

        self.excitation_filter
            .process_buffer(temp, temp_2, FilterMode::LowPass);

        for (aux_sample, temp_sample) in aux.iter_mut().zip(temp_2.iter()) {
            *aux_sample += *temp_sample;
        }

        let non_linearity = if structure < 0.24 {
            (structure - 0.24) * 4.166
        } else if structure > 0.26 {
            (structure - 0.26) * 1.35135
        } else {
            0.0
        };
        self.string
            .process(f0, non_linearity, brightness, damping, temp_2, out);
    }
}
