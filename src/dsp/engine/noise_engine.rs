//! Filtered noise.
//!
//! Variable-clock white noise processed by a resonant filter.
//!
//! Engine parameters:
//! - *HARMONICS:* filter response, from LP to BP to HP.
//! - *TIMBRE:* clock frequency.
//! - *MORPH:* filter resonance.
//!
//! *AUX* signal: variant employing two band-pass filters, with their separation
//! controlled by *HARMONICS*.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::dsp::allocate_buffer;
use crate::dsp::noise::clocked_noise::ClockedNoise;
use crate::stmlib::dsp::filter::{FilterMode, FrequencyApproximation, Svf};
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::sqrt;
use crate::stmlib::dsp::units::semitones_to_ratio;

#[derive(Debug)]
pub struct NoiseEngine<'a> {
    clocked_noise: [ClockedNoise; 2],
    lp_hp_filter: Svf,
    bp_filter: [Svf; 2],

    previous_f0: f32,
    previous_f1: f32,
    previous_q: f32,
    previous_mode: f32,

    temp_buffer: &'a mut [f32],
}

impl<'a> NoiseEngine<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            clocked_noise: [ClockedNoise::default(), ClockedNoise::default()],
            lp_hp_filter: Svf::default(),
            bp_filter: [Svf::default(), Svf::default()],
            previous_f0: 0.0,
            previous_f1: 0.0,
            previous_q: 0.0,
            previous_mode: 0.0,
            temp_buffer: allocate_buffer(buffer_allocator, block_size).unwrap(),
        }
    }
}

impl<'a> Engine for NoiseEngine<'a> {
    fn init(&mut self) {
        self.clocked_noise[0].init();
        self.clocked_noise[1].init();
        self.lp_hp_filter.init();
        self.bp_filter[0].init();
        self.bp_filter[1].init();

        self.previous_f0 = 0.0;
        self.previous_f1 = 0.0;
        self.previous_q = 0.0;
        self.previous_mode = 0.0;
    }

    #[inline]
    fn render(
        &mut self,
        parameters: &EngineParameters,
        out: &mut [f32],
        aux: &mut [f32],
        _already_enveloped: &mut bool,
    ) {
        let sustain = matches!(
            parameters.trigger,
            TriggerState::Unpatched | TriggerState::UnpatchedAutotriggered
        );

        let trigger = matches!(
            parameters.trigger,
            TriggerState::RisingEdge | TriggerState::UnpatchedAutotriggered
        );

        let f0 = note_to_frequency(parameters.note);
        let f1 = note_to_frequency(parameters.note + parameters.harmonics * 48.0 - 24.0);
        let clock_lowest_note = if sustain { 0.0 } else { -24.0 };
        let clock_f =
            note_to_frequency(parameters.timbre * (128.0 - clock_lowest_note) + clock_lowest_note);
        let q = 0.5 * semitones_to_ratio(parameters.morph * 120.0);
        let sync = trigger;
        self.clocked_noise[0].render(sync, clock_f, aux);
        self.clocked_noise[1].render(sync, clock_f * f1 / f0, self.temp_buffer);

        let mut f0_modulation = ParameterInterpolator::new(&mut self.previous_f0, f0, out.len());
        let mut f1_modulation = ParameterInterpolator::new(&mut self.previous_f1, f1, out.len());
        let mut q_modulation = ParameterInterpolator::new(&mut self.previous_q, q, out.len());
        let mut mode_modulation =
            ParameterInterpolator::new(&mut self.previous_mode, parameters.harmonics, out.len());

        let in_1 = aux;
        let in_2 = &self.temp_buffer;

        for (out_sample, (in_1_sample, in_2_sample)) in
            out.iter_mut().zip(in_1.iter_mut().zip(in_2.iter()))
        {
            let f0 = f0_modulation.next();
            let f1 = f1_modulation.next();
            let q = q_modulation.next();
            let gain = 1.0 / sqrt((0.5 + q) * 40.0 * f0);
            self.lp_hp_filter
                .set_f_q(f0, q, FrequencyApproximation::Accurate);
            self.bp_filter[0].set_f_q(f0, q, FrequencyApproximation::Accurate);
            self.bp_filter[1].set_f_q(f1, q, FrequencyApproximation::Accurate);

            let input_1 = *in_1_sample * gain;
            let input_2 = *in_2_sample * gain;
            self.lp_hp_filter.process_multimode_buffer(
                core::slice::from_ref(&input_1),
                core::slice::from_mut(out_sample),
                mode_modulation.next(),
            );
            *in_1_sample = self.bp_filter[0].process(input_1, FilterMode::BandPass)
                + self.bp_filter[1].process(input_2, FilterMode::BandPass);
        }
    }
}
