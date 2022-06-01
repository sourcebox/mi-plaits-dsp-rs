//! Swarm of sawtooths and sines.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{note_to_frequency, Engine, EngineParameters, TriggerState};
use crate::dsp::oscillator::sine_oscillator::FastSineOscillator;
use crate::dsp::resources::LUT_SINE;
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::polyblep::{next_blep_sample, this_blep_sample};
use crate::stmlib::dsp::units::semitones_to_ratio;
use crate::stmlib::dsp::{interpolate_wrap, one_pole};
use crate::stmlib::utils::random;

const NUM_SWARM_VOICES: usize = 8;
const MAX_FREQUENCY: f32 = 0.5;

#[derive(Debug, Default)]
pub struct SwarmEngine {
    swarm_voice: [SwarmVoice; NUM_SWARM_VOICES],
}

impl SwarmEngine {
    pub fn new() -> Self {
        Self {
            swarm_voice: [
                SwarmVoice::default(),
                SwarmVoice::default(),
                SwarmVoice::default(),
                SwarmVoice::default(),
                SwarmVoice::default(),
                SwarmVoice::default(),
                SwarmVoice::default(),
                SwarmVoice::default(),
            ],
        }
    }
}

impl Engine for SwarmEngine {
    fn init(&mut self) {
        let n = (NUM_SWARM_VOICES as i32 - 1) / 2;

        for i in 0..NUM_SWARM_VOICES as i32 {
            let rank = i.wrapping_sub(n) as f32 / n as f32;
            self.swarm_voice[i as usize].init(rank);
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
        let f0 = note_to_frequency(parameters.note);
        let control_rate = out.len() as f32;
        let density = note_to_frequency(parameters.timbre * 120.0) * 0.025 * control_rate;
        let spread = parameters.harmonics * parameters.harmonics * parameters.harmonics;
        let mut size_ratio = 0.25 * semitones_to_ratio((1.0 - parameters.morph) * 84.0);

        let sustain = matches!(
            parameters.trigger,
            TriggerState::Unpatched | TriggerState::UnpatchedAutotriggered
        );

        let trigger = matches!(
            parameters.trigger,
            TriggerState::RisingEdge | TriggerState::UnpatchedAutotriggered
        );

        let burst_mode = !(sustain);
        let start_burst = trigger;

        out.fill(0.0);
        aux.fill(0.0);

        for swarm_voice in &mut self.swarm_voice {
            swarm_voice.render(
                f0,
                density,
                burst_mode,
                start_burst,
                spread,
                size_ratio,
                out,
                aux,
            );
            size_ratio *= 0.97;
        }
    }
}

#[derive(Debug, Default)]
pub struct SwarmVoice {
    rank: f32,

    envelope: GrainEnvelope,
    saw: AdditiveSawOscillator,
    sine: FastSineOscillator,
}

impl SwarmVoice {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, rank: f32) {
        self.rank = rank;
        self.envelope.init();
        self.saw.init();
        self.sine.init();
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        mut f0: f32,
        density: f32,
        burst_mode: bool,
        start_burst: bool,
        spread: f32,
        size_ratio: f32,
        saw: &mut [f32],
        sine: &mut [f32],
    ) {
        self.envelope.step(density, burst_mode, start_burst);

        let scale = 1.0 / NUM_SWARM_VOICES as f32;
        let amplitude = self.envelope.amplitude(size_ratio) * scale;

        let expo_amount = self.envelope.frequency(size_ratio);
        f0 *= semitones_to_ratio(48.0 * expo_amount * spread * self.rank);

        let linear_amount = self.rank * (self.rank + 0.01) * spread * 0.25;
        f0 *= 1.0 + linear_amount;

        self.saw.render(f0, amplitude, saw);
        self.sine.render_add(f0, amplitude, sine);
    }
}

#[derive(Debug, Default)]
pub struct AdditiveSawOscillator {
    // Oscillator state.
    phase: f32,
    next_sample: f32,

    // For interpolation of parameters.
    frequency: f32,
    gain: f32,
}

impl AdditiveSawOscillator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.next_sample = 0.0;
        self.frequency = 0.01;
        self.gain = 0.0;
    }

    #[inline]
    pub fn render(&mut self, mut frequency: f32, level: f32, out: &mut [f32]) {
        if frequency >= MAX_FREQUENCY {
            frequency = MAX_FREQUENCY;
        }
        let mut fm = ParameterInterpolator::new(&mut self.frequency, frequency, out.len());
        let mut gain = ParameterInterpolator::new(&mut self.gain, level, out.len());

        let mut next_sample = self.next_sample;
        let mut phase = self.phase;

        for out_sample in out.iter_mut() {
            let mut this_sample = next_sample;
            next_sample = 0.0;

            let frequency = fm.next();

            phase += frequency;

            if phase >= 1.0 {
                phase -= 1.0;
                let t = phase / frequency;
                this_sample -= this_blep_sample(t);
                next_sample -= next_blep_sample(t);
            }

            next_sample += phase;
            *out_sample += (2.0 * this_sample - 1.0) * gain.next();
        }
        self.phase = phase;
        self.next_sample = next_sample;
    }
}

#[derive(Debug, Default)]
pub struct GrainEnvelope {
    from: f32,
    interval: f32,
    phase: f32,
    fm: f32,
    amplitude: f32,
    previous_size_ratio: f32,
    filter_coefficient: f32,
}

impl GrainEnvelope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.from = 0.0;
        self.interval = 1.0;
        self.phase = 1.0;
        self.fm = 0.0;
        self.amplitude = 0.5;
        self.previous_size_ratio = 0.0;
    }

    #[inline]
    pub fn step(&mut self, rate: f32, burst_mode: bool, start_burst: bool) {
        let mut randomize = false;

        if start_burst {
            self.phase = 0.5;
            self.fm = 16.0;
            randomize = true;
        } else {
            self.phase += rate * self.fm;
            if self.phase >= 1.0 {
                self.phase -= ((self.phase) as i32) as f32;
                randomize = true;
            }
        }

        if randomize {
            self.from += self.interval;
            self.interval = random::get_float() - self.from;
            // Randomize the duration of the grain.
            if burst_mode {
                self.fm *= 0.8 + 0.2 * random::get_float();
            } else {
                self.fm = 0.5 + 1.5 * random::get_float();
            }
        }
    }

    #[inline]
    pub fn frequency(&self, size_ratio: f32) -> f32 {
        // We approximate two overlapping grains of frequencies f1 and f2
        // By a continuous tone ramping from f1 to f2. This allows a continuous
        // transition between the "grain cloud" and "swarm of glissandi"
        // textures.
        if size_ratio < 1.0 {
            2.0 * (self.from + self.interval * self.phase) - 1.0
        } else {
            self.from
        }
    }

    #[inline]
    pub fn amplitude(&mut self, size_ratio: f32) -> f32 {
        let mut target_amplitude = 1.0;

        if size_ratio >= 1.0 {
            let mut phase = (self.phase - 0.5) * size_ratio;
            phase = phase.clamp(-1.0, 1.0);
            let e = interpolate_wrap(&LUT_SINE, 0.5 * phase + 1.25, 1024.0);
            target_amplitude = 0.5 * (e + 1.0);
        }

        if (size_ratio >= 1.0) ^ (self.previous_size_ratio >= 1.0) {
            self.filter_coefficient = 0.5;
        }

        self.filter_coefficient *= 0.95;

        self.previous_size_ratio = size_ratio;
        one_pole(
            &mut self.amplitude,
            target_amplitude,
            0.5 - self.filter_coefficient,
        );

        self.amplitude
    }
}
