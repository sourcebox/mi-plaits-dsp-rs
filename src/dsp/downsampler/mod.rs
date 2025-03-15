//! FIR Downsampler.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

const FIR_HALF_SIZE: usize = 4;
const FIR_COEFFICIENT: [f32; FIR_HALF_SIZE] = [0.02442415, 0.09297315, 0.16712938, 0.21547332];

#[derive(Debug)]
pub struct Downsampler<'a> {
    head: f32,
    tail: f32,
    state: &'a mut f32,
}

impl<'a> Downsampler<'a> {
    pub fn new(state: &'a mut f32) -> Self {
        Self {
            head: *state,
            tail: 0.0,
            state,
        }
    }

    #[inline]
    pub fn accumulate(&mut self, i: usize, sample: f32) {
        self.head += sample * FIR_COEFFICIENT[3 - (i & 3)];
        self.tail += sample * FIR_COEFFICIENT[i & 3];
    }

    #[inline]
    pub fn read(&mut self) -> f32 {
        let value = self.head;
        self.head = self.tail;
        self.tail = 0.0;

        value
    }
}

impl Drop for Downsampler<'_> {
    fn drop(&mut self) {
        *self.state = self.head;
    }
}
