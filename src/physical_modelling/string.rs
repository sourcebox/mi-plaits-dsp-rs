//! Comb filter / KS string. "Lite" version of the implementation used in Rings.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

use crate::resources::svf::LUT_SVF_SHIFT;
use crate::utils::delay_line::DelayLine;
use crate::utils::filter::{DcBlocker, FilterMode, FrequencyApproximation, Svf};
use crate::utils::parameter_interpolator::ParameterInterpolator;
use crate::utils::random;
use crate::utils::units::semitones_to_ratio;
use crate::utils::{crossfade, interpolate, one_pole};
use crate::SAMPLE_RATE;

pub const DELAY_LINE_SIZE: usize = 1024;

pub enum StringNonLinearity {
    CurvedBridge,
    Dispersion,
}

#[derive(Debug)]
pub struct String {
    string: DelayLine<f32, DELAY_LINE_SIZE>,
    stretch: DelayLine<f32, { DELAY_LINE_SIZE / 4 }>,

    iir_damping_filter: Svf,
    dc_blocker: DcBlocker,

    delay: f32,
    dispersion_noise: f32,
    curved_bridge: f32,

    // Very crappy linear interpolation upsampler used for low pitches that
    // do not fit the delay line. Rarely used.
    src_phase: f32,
    out_sample: [f32; 2],
}

impl Default for String {
    fn default() -> Self {
        Self::new()
    }
}

impl String {
    pub fn new() -> Self {
        Self {
            string: DelayLine::new(),
            stretch: DelayLine::new(),
            iir_damping_filter: Svf::default(),
            dc_blocker: DcBlocker::default(),
            delay: 0.0,
            dispersion_noise: 0.0,
            curved_bridge: 0.0,
            src_phase: 0.0,
            out_sample: [0.0; 2],
        }
    }

    pub fn reset(&mut self) {
        self.string.reset();
        self.stretch.reset();
        self.iir_damping_filter.init();
        self.dc_blocker.init(1.0 - 20.0 / SAMPLE_RATE);
        self.dispersion_noise = 0.0;
        self.curved_bridge = 0.0;
        self.out_sample[0] = 0.0;
        self.out_sample[1] = 0.0;
        self.src_phase = 0.0;
    }

    #[inline]
    pub fn process(
        &mut self,
        f0: f32,
        non_linearity_amount: f32,
        brightness: f32,
        damping: f32,
        in_: &[f32],
        out: &mut [f32],
    ) {
        if non_linearity_amount <= 0.0 {
            self.process_internal(
                f0,
                -non_linearity_amount,
                brightness,
                damping,
                in_,
                out,
                StringNonLinearity::CurvedBridge,
            );
        } else {
            self.process_internal(
                f0,
                non_linearity_amount,
                brightness,
                damping,
                in_,
                out,
                StringNonLinearity::Dispersion,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn process_internal(
        &mut self,
        f0: f32,
        non_linearity_amount: f32,
        mut brightness: f32,
        damping: f32,
        in_: &[f32],
        out: &mut [f32],
        non_linearity: StringNonLinearity,
    ) {
        let delay = (1.0 / f0).clamp(4.0, DELAY_LINE_SIZE as f32 - 4.0);

        // If there is not enough delay time in the delay line, we play at the
        // lowest possible note and we upsample on the fly with a shitty linear
        // interpolator. We don't care because it's a corner case (f0 < 11.7Hz)
        let mut src_ratio = delay * f0;
        if src_ratio >= 0.9999 {
            // When we are above 11.7 Hz, we make sure that the linear interpolator
            // does not get in the way.
            self.src_phase = 1.0;
            src_ratio = 1.0;
        }

        let mut damping_cutoff =
            f32::min(12.0 + damping * damping * 60.0 + brightness * 24.0, 84.0);
        let mut damping_f = f32::min(f0 * semitones_to_ratio(damping_cutoff), 0.499);

        // Crossfade to infinite decay.
        if damping >= 0.95 {
            let to_infinite = 20.0 * (damping - 0.95);
            brightness += to_infinite * (1.0 - brightness);
            damping_f += to_infinite * (0.4999 - damping_f);
            damping_cutoff += to_infinite * (128.0 - damping_cutoff);
        }

        self.iir_damping_filter
            .set_f_q(damping_f, 0.5, FrequencyApproximation::Fast);

        let damping_compensation = interpolate(&LUT_SVF_SHIFT, damping_cutoff, 1.0);

        // Linearly interpolate delay time.
        let mut delay_modulation =
            ParameterInterpolator::new(&mut self.delay, delay * damping_compensation, out.len());

        let stretch_point = non_linearity_amount * (2.0 - non_linearity_amount) * 0.225;
        let mut stretch_correction = (160.0 / SAMPLE_RATE) * delay;
        stretch_correction = stretch_correction.clamp(1.0, 2.1);

        let noise_amount_sqrt = if non_linearity_amount > 0.75 {
            4.0 * (non_linearity_amount - 0.75)
        } else {
            0.0
        };
        let noise_amount = noise_amount_sqrt * noise_amount_sqrt * 0.1;
        let noise_filter = 0.06 + 0.94 * brightness * brightness;

        let bridge_curving_sqrt = non_linearity_amount;
        let bridge_curving = bridge_curving_sqrt * bridge_curving_sqrt * 0.01;

        let ap_gain = -0.618 * non_linearity_amount / (0.15 + non_linearity_amount.abs());

        for (in_sample, out_sample) in in_.iter().zip(out.iter_mut()) {
            self.src_phase += src_ratio;
            if self.src_phase > 1.0 {
                self.src_phase -= 1.0;

                let mut delay = delay_modulation.next();
                let mut s;

                if matches!(non_linearity, StringNonLinearity::Dispersion) {
                    let noise = random::get_float() - 0.5;
                    one_pole(&mut self.dispersion_noise, noise, noise_filter);
                    delay *= 1.0 + self.dispersion_noise * noise_amount;
                } else {
                    delay *= 1.0 - self.curved_bridge * bridge_curving;
                }

                if matches!(non_linearity, StringNonLinearity::Dispersion) {
                    let ap_delay = delay * stretch_point;
                    let main_delay =
                        delay - ap_delay * (0.408 - stretch_point * 0.308) * stretch_correction;
                    if ap_delay >= 4.0 && main_delay >= 4.0 {
                        s = self.string.read_with_delay_frac(main_delay);
                        s = self.stretch.allpass(s, ap_delay as usize, ap_gain);
                    } else {
                        s = self.string.read_hermite(delay);
                    }
                } else {
                    s = self.string.read_hermite(delay);
                }

                if matches!(non_linearity, StringNonLinearity::CurvedBridge) {
                    let value = s.abs() - 0.025;
                    let sign = if s > 0.0 { 1.0 } else { -1.5 };
                    self.curved_bridge = (value.abs() + value) * sign;
                }

                s += (*in_sample).clamp(-20.0, 20.0);
                self.dc_blocker.process(core::slice::from_mut(&mut s));
                s = self.iir_damping_filter.process(s, FilterMode::LowPass);
                self.string.write(s);

                self.out_sample[1] = self.out_sample[0];
                self.out_sample[0] = s;
            }
            *out_sample += crossfade(self.out_sample[1], self.out_sample[0], self.src_phase);
        }
    }
}
