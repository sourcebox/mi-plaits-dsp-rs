//! Delay line.

// Based on MIT-licensed code (c) 2014 by Olivier Gillet (ol.gillet@gmail.com)

use num_traits::{FromPrimitive, Num, Signed, ToPrimitive};

#[derive(Debug)]
pub struct DelayLine<T, const MAX_DELAY: usize> {
    write_ptr: usize,
    delay: usize,
    line: [T; MAX_DELAY],
}

impl<T, const MAX_DELAY: usize> Default for DelayLine<T, MAX_DELAY>
where
    T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const MAX_DELAY: usize> DelayLine<T, MAX_DELAY>
where
    T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
{
    pub fn new() -> Self {
        Self {
            write_ptr: 0,
            delay: 1,
            line: [T::zero(); MAX_DELAY],
        }
    }

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

    pub fn max_delay(&self) -> usize {
        MAX_DELAY
    }

    #[inline]
    pub fn set_delay(&mut self, delay: usize) {
        self.delay = delay;
    }

    #[inline]
    pub fn write(&mut self, sample: T) {
        self.line[self.write_ptr] = sample;
        self.write_ptr = (self.write_ptr + MAX_DELAY - 1) % MAX_DELAY;
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

        let frac = (b - a).to_f32().unwrap_or_default() * delay_fractional;

        T::from_f32(a.to_f32().unwrap_or_default() + frac).unwrap_or_default()
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
        let c = T::from_f32((x1 - xm1).to_f32().unwrap_or_default() * 0.5).unwrap_or_default();
        let v = x0 - x1;
        let w = c + v;
        let a = T::from_f32(
            (w + v).to_f32().unwrap_or_default() + ((x2 - x0).to_f32().unwrap_or_default() * 0.5),
        )
        .unwrap_or_default();
        let b_neg = w + a;
        let f = delay_fractional;

        T::from_f32(
            (((a.to_f32().unwrap_or_default() * f) - b_neg.to_f32().unwrap_or_default()) * f
                + c.to_f32().unwrap_or_default())
                * f
                + x0.to_f32().unwrap_or_default(),
        )
        .unwrap_or_default()
    }
}
