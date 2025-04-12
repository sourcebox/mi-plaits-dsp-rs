//! Zero-delay-feedback filters (one pole and SVF). Naive SVF.

// Based on MIT-licensed code (c) 2014 by Olivier Gillet (ol.gillet@gmail.com)

#[allow(unused_imports)]
use num_traits::float::Float;

#[derive(Debug)]
pub enum FilterMode {
    LowPass,
    BandPass,
    BandPassNormalized,
    HighPass,
}

#[derive(Debug)]
pub enum FrequencyApproximation {
    Exact,
    Accurate,
    Fast,
    Dirty,
}

const M_PI_F: f32 = core::f32::consts::PI;
const M_PI_POW_2: f32 = M_PI_F * M_PI_F;
const M_PI_POW_3: f32 = M_PI_POW_2 * M_PI_F;
const M_PI_POW_5: f32 = M_PI_POW_3 * M_PI_POW_2;
const M_PI_POW_7: f32 = M_PI_POW_5 * M_PI_POW_2;
const M_PI_POW_9: f32 = M_PI_POW_7 * M_PI_POW_2;
const M_PI_POW_11: f32 = M_PI_POW_9 * M_PI_POW_2;

#[derive(Debug, Default)]
pub struct DcBlocker {
    pole: f32,
    x: f32,
    y: f32,
}

impl DcBlocker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, pole: f32) {
        self.x = 0.0;
        self.y = 0.0;
        self.pole = pole;
    }

    #[inline]
    pub fn process(&mut self, in_out: &mut [f32]) {
        let mut x = self.x;
        let mut y = self.y;
        let pole = self.pole;
        for sample in in_out.iter_mut() {
            let old_x = x;
            x = *sample;
            y = y * pole + x - old_x;
            *sample = y
        }
        self.x = x;
        self.y = y;
    }
}

#[derive(Debug, Default)]
pub struct OnePole {
    g: f32,
    gi: f32,
    state: f32,
}

impl OnePole {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.set_f(0.01, FrequencyApproximation::Dirty);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.state = 0.0;
    }

    #[inline]
    #[allow(clippy::excessive_precision)]
    pub fn tan(f: f32, approximation: FrequencyApproximation) -> f32 {
        match approximation {
            FrequencyApproximation::Exact => {
                // Clip coefficient to about 100.
                let f = if f < 0.497 { f } else { 0.497 };
                (M_PI_F * f).tan()
            }
            FrequencyApproximation::Dirty => {
                // Optimized for frequencies below 8kHz.
                const A: f32 = 3.736e-01 * M_PI_POW_3;
                f * (M_PI_F + A * f * f)
            }
            FrequencyApproximation::Fast => {
                // The usual tangent approximation uses 3.1755e-01 and 2.033e-01, but
                // the coefficients used here are optimized to minimize error for the
                // 16Hz to 16kHz range, with a sample rate of 48kHz.
                const A: f32 = 3.260e-01 * M_PI_POW_3;
                const B: f32 = 1.823e-01 * M_PI_POW_5;
                let f2 = f * f;
                f * (M_PI_F + f2 * (A + B * f2))
            }
            FrequencyApproximation::Accurate => {
                // These coefficients don't need to be tweaked for the audio range.
                const A: f32 = 3.333314036e-01 * M_PI_POW_3;
                const B: f32 = 1.333923995e-01 * M_PI_POW_5;
                const C: f32 = 5.33740603e-02 * M_PI_POW_7;
                const D: f32 = 2.900525e-03 * M_PI_POW_9;
                const E: f32 = 9.5168091e-03 * M_PI_POW_11;
                let f2 = f * f;
                f * (M_PI_F + f2 * (A + f2 * (B + f2 * (C + f2 * (D + f2 * E)))))
            }
        }
    }

    #[inline]
    pub fn set_f(&mut self, f: f32, approximation: FrequencyApproximation) {
        self.g = Self::tan(f, approximation);
        self.gi = 1.0 / (1.0 + self.g);
    }

    #[inline]
    pub fn process(&mut self, in_: f32, mode: FilterMode) -> f32 {
        let lp = (self.g * in_ + self.state) * self.gi;
        self.state = self.g * (in_ - lp) + lp;
        match mode {
            FilterMode::LowPass => lp,
            FilterMode::HighPass => in_ - lp,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Default)]
pub struct Svf {
    g: f32,
    r: f32,
    h: f32,
    state_1: f32,
    state_2: f32,
}

impl Svf {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.set_f_q(0.01, 100.0, FrequencyApproximation::Dirty);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.state_1 = 0.0;
        self.state_2 = 0.0;
    }

    /// Copy settings from another filter.
    #[inline]
    pub fn set(&mut self, f: &Self) {
        self.g = f.g();
        self.r = f.r();
        self.h = f.h();
    }

    /// Set all parameters from LUT.
    #[inline]
    pub fn set_g_r_h(&mut self, g: f32, r: f32, h: f32) {
        self.g = g;
        self.r = r;
        self.h = h;
    }

    /// Set frequency and resonance coefficients from LUT, adjust remaining parameter.
    #[inline]
    pub fn set_g_r(&mut self, g: f32, r: f32) {
        self.g = g;
        self.r = r;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
    }

    /// Set frequency from LUT, resonance in true units, adjust the rest.
    #[inline]
    pub fn set_g_q(&mut self, g: f32, resonance: f32) {
        self.g = g;
        self.r = 1.0 / resonance;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
    }

    /// Set frequency and resonance from true units. Various approximations
    /// are available to avoid the cost of tanf.
    #[inline]
    pub fn set_f_q(&mut self, f: f32, resonance: f32, approximation: FrequencyApproximation) {
        self.g = OnePole::tan(f, approximation);
        self.r = 1.0 / resonance;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
    }

    #[inline]
    pub fn process(&mut self, in_: f32, mode: FilterMode) -> f32 {
        let hp = (in_ - self.r * self.state_1 - self.g * self.state_1 - self.state_2) * self.h;
        let bp = self.g * hp + self.state_1;
        self.state_1 = self.g * hp + bp;
        let lp = self.g * bp + self.state_2;
        self.state_2 = self.g * bp + lp;

        match mode {
            FilterMode::LowPass => lp,
            FilterMode::BandPass => bp,
            FilterMode::BandPassNormalized => bp * self.r,
            FilterMode::HighPass => hp,
        }
    }

    #[inline]
    pub fn process_dual(
        &mut self,
        in_: f32,
        out_1: &mut f32,
        out_2: &mut f32,
        mode_1: FilterMode,
        mode_2: FilterMode,
    ) {
        let hp = (in_ - self.r * self.state_1 - self.g * self.state_1 - self.state_2) * self.h;
        let bp = self.g * hp + self.state_1;
        self.state_1 = self.g * hp + bp;
        let lp = self.g * bp + self.state_2;
        self.state_2 = self.g * bp + lp;

        match mode_1 {
            FilterMode::LowPass => {
                *out_1 = lp;
            }
            FilterMode::BandPass => {
                *out_1 = bp;
            }
            FilterMode::BandPassNormalized => {
                *out_1 = bp * self.r;
            }
            FilterMode::HighPass => {
                *out_1 = hp;
            }
        }

        match mode_2 {
            FilterMode::LowPass => {
                *out_2 = lp;
            }
            FilterMode::BandPass => {
                *out_2 = bp;
            }
            FilterMode::BandPassNormalized => {
                *out_2 = bp * self.r;
            }
            FilterMode::HighPass => {
                *out_2 = hp;
            }
        }
    }

    #[inline]
    pub fn process_buffer(&mut self, in_: &[f32], out: &mut [f32], mode: FilterMode) {
        let mut state_1 = self.state_1;
        let mut state_2 = self.state_2;

        let iter = in_.iter().zip(out.iter_mut());

        for (sample_in, sample_out) in iter {
            let hp = (*sample_in - self.r * state_1 - self.g * state_1 - state_2) * self.h;
            let bp = self.g * hp + state_1;
            state_1 = self.g * hp + bp;
            let lp = self.g * bp + state_2;
            state_2 = self.g * bp + lp;

            match mode {
                FilterMode::LowPass => {
                    *sample_out = lp;
                }
                FilterMode::BandPass => {
                    *sample_out = bp;
                }
                FilterMode::BandPassNormalized => {
                    *sample_out = bp * self.r;
                }
                FilterMode::HighPass => {
                    *sample_out = hp;
                }
            }
        }

        self.state_1 = state_1;
        self.state_2 = state_2;
    }

    #[inline]
    pub fn process_add_buffer(
        &mut self,
        in_: &[f32],
        out: &mut [f32],
        gain: f32,
        mode: FilterMode,
    ) {
        let mut state_1 = self.state_1;
        let mut state_2 = self.state_2;

        let iter = in_.iter().zip(out.iter_mut());

        for (sample_in, sample_out) in iter {
            let hp = (*sample_in - self.r * state_1 - self.g * state_1 - state_2) * self.h;
            let bp = self.g * hp + state_1;
            state_1 = self.g * hp + bp;
            let lp = self.g * bp + state_2;
            state_2 = self.g * bp + lp;

            match mode {
                FilterMode::LowPass => {
                    *sample_out = lp * gain;
                }
                FilterMode::BandPass => {
                    *sample_out = bp * gain;
                }
                FilterMode::BandPassNormalized => {
                    *sample_out = bp * self.r * gain;
                }
                FilterMode::HighPass => {
                    *sample_out = hp * gain;
                }
            }
        }

        self.state_1 = state_1;
        self.state_2 = state_2;
    }

    #[inline]
    pub fn process_stride_buffer(
        &mut self,
        in_: &[f32],
        out: &mut [f32],
        stride: usize,
        mode: FilterMode,
    ) {
        let mut state_1 = self.state_1;
        let mut state_2 = self.state_2;

        let iter = in_.iter().zip(out.iter_mut()).step_by(stride);

        for (sample_in, sample_out) in iter {
            let hp = (*sample_in - self.r * state_1 - self.g * state_1 - state_2) * self.h;
            let bp = self.g * hp + state_1;
            state_1 = self.g * hp + bp;
            let lp = self.g * bp + state_2;
            state_2 = self.g * bp + lp;

            match mode {
                FilterMode::LowPass => {
                    *sample_out = lp;
                }
                FilterMode::BandPass => {
                    *sample_out = bp;
                }
                FilterMode::BandPassNormalized => {
                    *sample_out = bp * self.r;
                }
                FilterMode::HighPass => {
                    *sample_out = hp;
                }
            }
        }

        self.state_1 = state_1;
        self.state_2 = state_2;
    }

    #[inline]
    pub fn process_multimode_buffer(&mut self, in_: &[f32], out: &mut [f32], mode: f32) {
        let mut state_1 = self.state_1;
        let mut state_2 = self.state_2;
        let hp_gain = if mode < 0.5 {
            -mode * 2.0
        } else {
            -2.0 + mode * 2.0
        };
        let lp_gain = if mode < 0.5 { 1.0 - mode * 2.0 } else { 0.0 };
        let bp_gain = if mode < 0.5 { 0.0 } else { mode * 2.0 - 1.0 };

        let iter = in_.iter().zip(out.iter_mut());

        for (sample_in, sample_out) in iter {
            let hp = (*sample_in - self.r * state_1 - self.g * state_1 - state_2) * self.h;
            let bp = self.g * hp + state_1;
            state_1 = self.g * hp + bp;
            let lp = self.g * bp + state_2;
            state_2 = self.g * bp + lp;
            *sample_out = hp_gain * hp + bp_gain * bp + lp_gain * lp;
        }

        self.state_1 = state_1;
        self.state_2 = state_2;
    }

    #[inline]
    pub fn process_add_dual_buffer(
        &mut self,
        in_: &[f32],
        out_1: &mut [f32],
        out_2: &mut [f32],
        gain_1: f32,
        gain_2: f32,
        mode: FilterMode,
    ) {
        let mut state_1 = self.state_1;
        let mut state_2 = self.state_2;

        let iter = in_.iter().zip(out_1.iter_mut().zip(out_2.iter_mut()));

        for (sample_in, (sample_out_1, sample_out_2)) in iter {
            let hp = (*sample_in - self.r * state_1 - self.g * state_1 - state_2) * self.h;
            let bp = self.g * hp + state_1;
            state_1 = self.g * hp + bp;
            let lp = self.g * bp + state_2;
            state_2 = self.g * bp + lp;

            match mode {
                FilterMode::LowPass => {
                    *sample_out_1 = lp * gain_1;
                    *sample_out_2 = lp * gain_2;
                }
                FilterMode::BandPass => {
                    *sample_out_1 = bp * gain_1;
                    *sample_out_2 = bp * gain_2;
                }
                FilterMode::BandPassNormalized => {
                    *sample_out_1 = bp * self.r * gain_1;
                    *sample_out_2 = bp * self.r * gain_2;
                }
                FilterMode::HighPass => {
                    *sample_out_1 = hp * gain_1;
                    *sample_out_2 = hp * gain_2;
                }
            }
        }

        self.state_1 = state_1;
        self.state_2 = state_2;
    }

    #[inline]
    pub fn g(&self) -> f32 {
        self.g
    }

    #[inline]
    pub fn r(&self) -> f32 {
        self.r
    }

    #[inline]
    pub fn h(&self) -> f32 {
        self.h
    }
}

/// Naive Chamberlin SVF.
#[derive(Debug, Default)]
pub struct NaiveSvf {
    f: f32,
    damp: f32,
    lp: f32,
    bp: f32,
}

impl NaiveSvf {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.set_f_q(0.01, 100.0, FrequencyApproximation::Dirty);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.lp = 0.0;
        self.bp = 0.0;
    }

    #[inline]
    pub fn set_f_q(&mut self, f: f32, resonance: f32, approximation: FrequencyApproximation) {
        match approximation {
            FrequencyApproximation::Exact => {
                let f = if f < 0.497 { f } else { 0.497 };
                self.f = 2.0 * (M_PI_F * f).sin();
            }
            _ => {
                let f = if f < 0.158 { f } else { 0.158 };
                self.f = 2.0 * M_PI_F * f;
            }
        }
        self.damp = 1.0 / resonance;
    }

    #[inline]
    pub fn process(&mut self, in_: f32, mode: FilterMode) -> f32 {
        let bp_normalized = self.bp * self.damp;
        let notch = in_ - bp_normalized;
        self.lp += self.f * self.bp;
        let hp = notch - self.lp;
        self.bp += self.f * hp;

        match mode {
            FilterMode::LowPass => self.lp,
            FilterMode::BandPass => self.bp,
            FilterMode::BandPassNormalized => bp_normalized,
            FilterMode::HighPass => hp,
        }
    }

    #[inline]
    pub fn lp(&self) -> f32 {
        self.lp
    }

    #[inline]
    pub fn bp(&self) -> f32 {
        self.bp
    }

    #[inline]
    pub fn process_buffer(&mut self, in_: &[f32], out: &mut [f32], mode: FilterMode) {
        let mut lp = self.lp;
        let mut bp = self.bp;

        let iter = in_.iter().zip(out.iter_mut());

        for (sample_in, sample_out) in iter {
            let bp_normalized = bp * self.damp;
            let notch = *sample_in - bp_normalized;
            lp += self.f * bp;
            let hp = notch - lp;
            bp += self.f * hp;

            *sample_out = match mode {
                FilterMode::LowPass => lp,
                FilterMode::BandPass => bp,
                FilterMode::BandPassNormalized => bp_normalized,
                FilterMode::HighPass => hp,
            }
        }

        self.lp = lp;
        self.bp = bp;
    }

    #[inline]
    pub fn split(&mut self, in_: &[f32], low: &mut [f32], high: &mut [f32]) {
        let mut lp = self.lp;
        let mut bp = self.bp;

        let iter = in_.iter().zip(low.iter_mut().zip(high.iter_mut()));

        for (sample_in, (sample_low, sample_high)) in iter {
            let bp_normalized = bp * self.damp;
            let notch = *sample_in - bp_normalized;
            lp += self.f * bp;
            let hp = notch - lp;
            bp += self.f * hp;
            *sample_low = lp;
            *sample_high = hp;
        }

        self.lp = lp;
        self.bp = bp;
    }

    #[inline]
    pub fn process_decimate(
        &mut self,
        in_: &[f32],
        out: &mut [f32],
        decimate: usize,
        mode: FilterMode,
    ) {
        let mut lp = self.lp;
        let mut bp = self.bp;
        let mut n = decimate - 1;

        let iter = in_.iter().zip(out.iter_mut());

        for (sample_in, sample_out) in iter {
            let bp_normalized = bp * self.damp;
            let notch = *sample_in - bp_normalized;
            lp += self.f * bp;
            let hp = notch - lp;
            bp += self.f * hp;

            n += 1;

            if n == decimate {
                *sample_out = match mode {
                    FilterMode::LowPass => lp,
                    FilterMode::BandPass => bp,
                    FilterMode::BandPassNormalized => bp_normalized,
                    FilterMode::HighPass => hp,
                };
                n = 0;
            }
        }

        self.lp = lp;
        self.bp = bp;
    }
}

/// Modified Chamberlin SVF (Duane K. Wise)
/// <http://www.dafx.ca/proceedings/papers/p_053.pdf>
#[derive(Debug, Default)]
pub struct ModifiedSvf {
    f: f32,
    fq: f32,
    x: f32,
    lp: f32,
    bp: f32,
}

impl ModifiedSvf {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.reset();
    }

    pub fn reset(&mut self) {
        self.lp = 0.0;
        self.bp = 0.0;
    }

    #[inline]
    pub fn set_f_fq(&mut self, f: f32, fq: f32) {
        self.f = f;
        self.fq = fq;
        self.x = 0.0;
    }

    #[inline]
    pub fn process(&mut self, in_: &[f32], out: &mut [f32], mode: FilterMode) {
        let mut lp = self.lp;
        let mut bp = self.bp;
        let mut x = self.x;
        let fq = self.fq;
        let f = self.f;

        let iter = in_.iter().zip(out.iter_mut());

        for (sample_in, sample_out) in iter {
            lp += f * bp;
            bp += -fq * bp - f * lp + *sample_in;
            match mode {
                FilterMode::BandPass | FilterMode::BandPassNormalized => {
                    bp += x;
                }
                _ => {}
            }

            x = *sample_in;

            *sample_out = match mode {
                FilterMode::LowPass => lp * f,
                FilterMode::BandPass => bp * f,
                FilterMode::BandPassNormalized => bp * fq,
                FilterMode::HighPass => x - lp * f - bp * fq,
            }
        }

        self.lp = lp;
        self.bp = bp;
        self.x = x;
    }
}

/// Two passes of modified Chamberlin SVF with the same coefficients -
/// to implement Linkwitz–Riley (Butterworth squared) crossover filters.
#[derive(Debug, Default)]
pub struct CrossoverSvf {
    f: f32,
    fq: f32,
    x: [f32; 2],
    lp: [f32; 2],
    bp: [f32; 2],
}

impl CrossoverSvf {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.reset();
    }

    pub fn reset(&mut self) {
        self.lp[0] = 0.0;
        self.bp[0] = 0.0;
        self.lp[1] = 0.0;
        self.bp[1] = 0.0;
        self.x[0] = 0.0;
        self.x[1] = 0.0;
    }

    #[inline]
    pub fn set_f_fq(&mut self, f: f32, fq: f32) {
        self.f = f;
        self.fq = fq;
    }

    #[inline]
    pub fn process(&mut self, in_: &[f32], out: &mut [f32], mode: FilterMode) {
        let mut lp_1 = self.lp[0];
        let mut bp_1 = self.bp[0];
        let mut lp_2 = self.lp[1];
        let mut bp_2 = self.bp[1];
        let mut x_1 = self.x[0];
        let mut x_2 = self.x[1];
        let fq = self.fq;
        let f = self.f;

        let iter = in_.iter().zip(out.iter_mut());

        for (sample_in, sample_out) in iter {
            lp_1 += f * bp_1;
            bp_1 += -fq * bp_1 - f * lp_1 + *sample_in;
            match mode {
                FilterMode::BandPass | FilterMode::BandPassNormalized => {
                    bp_1 += x_1;
                }
                _ => {}
            }

            x_1 = *sample_in;

            let y = match mode {
                FilterMode::LowPass => lp_1 * f,
                FilterMode::BandPass => bp_1 * f,
                FilterMode::BandPassNormalized => bp_1 * fq,
                FilterMode::HighPass => x_1 - lp_1 * f - bp_1 * fq,
            };

            lp_2 += f * bp_2;
            bp_2 += -fq * bp_2 - f * lp_2 + y;

            match mode {
                FilterMode::BandPass | FilterMode::BandPassNormalized => {
                    bp_2 += x_2;
                }
                _ => {}
            }

            x_2 = y;

            *sample_out = match mode {
                FilterMode::LowPass => lp_2 * f,
                FilterMode::BandPass => bp_2 * f,
                FilterMode::BandPassNormalized => bp_2 * fq,
                FilterMode::HighPass => x_2 - lp_2 * f - bp_2 * fq,
            };
        }

        self.lp[0] = lp_1;
        self.bp[0] = bp_1;
        self.lp[1] = lp_2;
        self.bp[1] = bp_2;
        self.x[0] = x_1;
        self.x[1] = x_2;
    }
}
