//! Sample rate reducer.

// Based on MIT-licensed code (c) 2014 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::stmlib::dsp::polyblep::{next_blep_sample, this_blep_sample};

#[derive(Debug, Default)]
pub struct SampleRateReducer {
    phase: f32,
    sample: f32,
    previous_sample: f32,
    next_sample: f32,
}

impl SampleRateReducer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.phase = 0.0;
        self.sample = 0.0;
        self.previous_sample = 0.0;
        self.next_sample = 0.0;
    }

    #[inline]
    pub fn process(
        &mut self,
        mut frequency: f32,
        in_out: &mut [f32],
        optimized_handling_of_special_cases: bool,
    ) {
        if optimized_handling_of_special_cases {
            // Use fast specialized implementations for target rates close to
            // the original rates. Caveats:
            // - The size argument must be a multiple of 4.
            // - There will be a transition glitch between the "optimized" and
            // the
            //   "common case" code, so don't use this when frequency is
            //   modulated!
            // - The optimized code is not a truly variable reclocking, instead,
            //   this is a crossfade between reclocking at SR / 2N and SR / N.
            if frequency >= 1.0 {
                return;
            } else if frequency >= 0.5 {
                self.process_half(2.0 - 2.0 * frequency, in_out);
                return;
            } else if frequency >= 0.25 {
                self.process_quarter(2.0 - 4.0 * frequency, in_out);
                return;
            }
        } else {
            frequency = frequency.clamp(0.0, 1.0);
        }

        let mut previous_sample = self.previous_sample;
        let mut next_sample = self.next_sample;
        let mut sample = self.sample;
        let mut phase = self.phase;

        for in_out_sample in in_out.iter_mut() {
            let mut this_sample = next_sample;
            next_sample = 0.0;
            phase += frequency;
            if phase >= 1.0 {
                phase -= 1.0;
                let t = phase / frequency;
                // t = 0: the transition occurred right at this sample.
                // t = 1: the transition occurred at the previous sample.
                // Use linear interpolation to recover the fractional sample.
                let new_sample = previous_sample + (*in_out_sample - previous_sample) * (1.0 - t);
                let discontinuity = new_sample - sample;
                this_sample += discontinuity * this_blep_sample(t);
                next_sample += discontinuity * next_blep_sample(t);
                sample = new_sample;
            }
            next_sample += sample;
            previous_sample = *in_out_sample;
            *in_out_sample = this_sample;
        }

        self.phase = phase;
        self.next_sample = next_sample;
        self.sample = sample;
        self.previous_sample = previous_sample;
    }

    #[inline]
    fn process_half(&mut self, amount: f32, in_out: &mut [f32]) {
        // assert(size % 2 == 0);
        let mut size = in_out.len();
        let mut in_out_index = 0;

        while size > 0 {
            in_out[in_out_index + 1] += (in_out[in_out_index] - in_out[in_out_index + 1]) * amount;
            in_out_index += 2;
            size -= 2;
        }

        self.sample = in_out[in_out_index - 1];
        self.next_sample = self.sample;
        self.previous_sample = self.sample;
    }

    #[inline]
    fn process_quarter(&mut self, amount: f32, in_out: &mut [f32]) {
        // assert(size % 4 == 0);
        let mut size = in_out.len();
        let mut in_out_index = 0;

        while size > 0 {
            in_out[in_out_index + 1] = in_out[in_out_index];
            in_out[in_out_index + 2] += (in_out[in_out_index] - in_out[in_out_index + 2]) * amount;
            in_out[in_out_index + 3] = in_out[in_out_index + 2];
            in_out_index += 4;
            size -= 4;
        }
        self.sample = in_out[in_out_index - 1];
        self.next_sample = self.sample;
        self.previous_sample = self.sample;
    }
}
