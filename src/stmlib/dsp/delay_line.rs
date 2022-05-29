//! Delay line.

// Based on MIT-licensed code (c) 2014 by Olivier Gillet (ol.gillet@gmail.com)

use num_traits::{Num, Signed};

#[derive(Debug)]
pub struct DelayLine<T, const MAX_DELAY: usize> {
    write_ptr: usize,
    delay: usize,
    line: [T; MAX_DELAY],
}

impl<T, const MAX_DELAY: usize> DelayLine<T, MAX_DELAY>
where
    T: Copy + Num + Signed + From<f32>,
{
    pub fn init(&mut self) {
        self.reset();
    }

    pub fn reset(&mut self) {
        for elem in self.line.iter_mut() {
            *elem = T::zero();
        }
        self.delay = 1;
        self.write_ptr = 0;
    }

    #[inline]
    pub fn set_delay(&mut self, delay: usize) {
        self.delay = delay;
    }

    #[inline]
    pub fn write(&mut self, sample: T) {
        self.line[self.write_ptr] = sample;
        self.write_ptr = (self.write_ptr - 1 + MAX_DELAY) % MAX_DELAY;
    }

    #[inline]
    pub fn allpass(&mut self, sample: T, delay: usize, coefficient: T) -> T {
        let read = self.line[(self.write_ptr + delay) % MAX_DELAY];
        let write = sample + coefficient * read;
        self.write(write);

        -write * coefficient + read
    }

    #[inline]
    pub fn write_read(&mut self, sample: T, delay: f32) -> T {
        self.write(sample);
        self.read_with_delay_frac(delay)
    }

    #[inline]
    pub fn read(&self) -> T {
        self.line[(self.write_ptr + self.delay) % MAX_DELAY]
    }

    #[inline]
    pub fn read_with_delay(&self, delay: usize) -> T {
        self.line[(self.write_ptr + delay) % MAX_DELAY]
    }

    #[inline]
    pub fn read_with_delay_frac(&self, delay: f32) -> T {
        let delay_integral = delay as usize;
        let delay_fractional = delay - (delay_integral as f32);
        let a = self.line[(self.write_ptr + delay_integral) % MAX_DELAY];
        let b = self.line[(self.write_ptr + delay_integral + 1) % MAX_DELAY];

        a + (b - a) * delay_fractional.into()
    }

    #[inline]
    pub fn read_hermite(&self, delay: f32) -> T {
        let delay_integral = delay as usize;
        let delay_fractional = delay - (delay_integral as f32);
        let t = self.write_ptr + delay_integral + MAX_DELAY;
        let xm1 = self.line[(t - 1) % MAX_DELAY];
        let x0 = self.line[(t) % MAX_DELAY];
        let x1 = self.line[(t + 1) % MAX_DELAY];
        let x2 = self.line[(t + 2) % MAX_DELAY];
        let c = (x1 - xm1) * 0.5.into();
        let v = x0 - x1;
        let w = c + v;
        let a = w + v + (x2 - x0) * 0.5.into();
        let b_neg = w + a;
        let f = delay_fractional.into();

        (((a * f) - b_neg) * f + c) * f + x0
    }
}
