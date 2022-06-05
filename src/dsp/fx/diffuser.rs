//! Granular diffuser.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use num_traits::{FromPrimitive, Num, Signed, ToPrimitive};

use crate::dsp::SAMPLE_RATE;
use crate::stmlib::dsp::clip_16;
use crate::stmlib::dsp::cosine_oscillator::{CosineOscillator, CosineOscillatorMode};
use crate::stmlib::dsp::delay_line::DelayLine;

#[derive(Debug, Default)]
pub struct Diffuser {
    ap1: DelayLine<i16, 126>,
    ap2: DelayLine<i16, 180>,
    ap3: DelayLine<i16, 269>,
    ap4: DelayLine<i16, 444>,
    dapa: DelayLine<i16, 1653>,
    dapb: DelayLine<i16, 2010>,
    del: DelayLine<i16, 3411>,

    engine: Engine,
    lp_decay: f32,
}

impl Diffuser {
    pub fn new() -> Self {
        Self {
            ap1: DelayLine::new(),
            ap2: DelayLine::new(),
            ap3: DelayLine::new(),
            ap4: DelayLine::new(),
            dapa: DelayLine::new(),
            dapb: DelayLine::new(),
            del: DelayLine::new(),

            engine: Engine::new(),
            lp_decay: 0.0,
        }
    }

    pub fn init(&mut self) {
        self.engine.set_lfo_frequency(0.3 / SAMPLE_RATE);
        self.lp_decay = 0.0;
    }

    pub fn clear(&mut self) {
        self.ap1.reset();
        self.ap2.reset();
        self.ap3.reset();
        self.ap4.reset();
        self.dapa.reset();
        self.dapb.reset();
        self.del.reset();
        self.engine.clear();
    }

    #[inline]
    pub fn process(&mut self, amount: f32, rt: f32, in_out: &mut [f32]) {
        let mut c = Context::new();

        let kap = 0.625;
        let klp = 0.75;
        let mut lp = self.lp_decay;

        for in_out_sample in in_out.iter_mut() {
            self.engine.start(&mut c);

            c.read(*in_out_sample);
            c.read_line(&mut self.ap1, kap);
            c.write_all_pass(&mut self.ap1, -kap);
            c.read_line(&mut self.ap2, kap);
            c.write_all_pass(&mut self.ap2, -kap);
            c.read_line(&mut self.ap3, kap);
            c.write_all_pass(&mut self.ap3, -kap);
            c.interpolate(&mut self.ap4, 400.0, 43.0, kap);
            c.write_all_pass(&mut self.ap4, -kap);
            c.interpolate(&mut self.del, 3070.0, 340.0, rt);
            c.lp(&mut lp, klp);
            c.read_line(&mut self.dapa, -kap);
            c.write_all_pass(&mut self.dapa, kap);
            c.read_line(&mut self.dapb, kap);
            c.write_all_pass(&mut self.dapb, -kap);
            c.write_line(&mut self.del, 2.0);

            let mut wet = 0.0;
            c.write_with_scale(&mut wet, 0.0);

            *in_out_sample += amount * (wet - *in_out_sample);
        }

        self.lp_decay = lp;
    }
}

#[derive(Debug, Default)]
struct Engine {
    lfo: CosineOscillator,
    write_ptr: i32,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.write_ptr = 0;
    }

    pub fn set_lfo_frequency(&mut self, frequency: f32) {
        self.lfo
            .init(frequency * 32.0, CosineOscillatorMode::Approximate);
    }

    pub fn start(&mut self, c: &mut Context) {
        self.write_ptr -= 1;

        if self.write_ptr < 0 {
            self.write_ptr += 8192;
        }

        c.accumulator = 0.0;
        c.previous_read = 0.0;

        if (self.write_ptr & 31) == 0 {
            c.lfo_value = self.lfo.next();
        } else {
            c.lfo_value = self.lfo.value();
        }
    }
}

#[derive(Debug, Default)]
struct Context {
    accumulator: f32,
    previous_read: f32,
    lfo_value: f32,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, value: f32) {
        self.accumulator = value;
    }

    pub fn read_with_scale(&mut self, value: f32, scale: f32) {
        self.accumulator += value * scale;
    }

    pub fn read(&mut self, value: f32) {
        self.accumulator += value;
    }

    pub fn write(&mut self, value: &mut f32) {
        *value = self.accumulator;
    }

    pub fn write_with_scale(&mut self, value: &mut f32, scale: f32) {
        *value = self.accumulator;
        self.accumulator *= scale;
    }

    pub fn write_line<T, const SIZE: usize>(&mut self, line: &mut DelayLine<T, SIZE>, scale: f32)
    where
        T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
    {
        let w = compress(self.accumulator);
        line.write(T::from_i16(w).unwrap_or_default());
        self.accumulator *= scale;
    }

    pub fn write_all_pass<T, const SIZE: usize>(
        &mut self,
        line: &mut DelayLine<T, SIZE>,
        scale: f32,
    ) where
        T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
    {
        self.write_line(line, scale);
        self.accumulator += self.previous_read;
    }

    pub fn read_line<T, const SIZE: usize>(&mut self, line: &mut DelayLine<T, SIZE>, scale: f32)
    where
        T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
    {
        let r = line.read_with_delay(line.max_delay());
        let r_f = decompress(r.to_i16().unwrap_or_default());
        self.previous_read = r_f;
        self.accumulator += r_f * scale;
    }

    pub fn lp(&mut self, state: &mut f32, coefficient: f32) {
        *state += coefficient * (self.accumulator - *state);
        self.accumulator = *state;
    }

    pub fn hp(&mut self, state: &mut f32, coefficient: f32) {
        *state += coefficient * (self.accumulator - *state);
        self.accumulator -= *state;
    }

    pub fn interpolate<T, const SIZE: usize>(
        &mut self,
        line: &mut DelayLine<T, SIZE>,
        mut offset: f32,
        amplitude: f32,
        scale: f32,
    ) where
        T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
    {
        offset += amplitude * self.lfo_value;
        let x = decompress(
            line.read_with_delay_frac((line.max_delay()) as f32 + offset)
                .to_i16()
                .unwrap_or_default(),
        );
        self.previous_read = x;
        self.accumulator += x * scale;
    }
}

fn decompress(value: i16) -> f32 {
    value as f32 / 4096.0
}

fn compress(value: f32) -> i16 {
    clip_16((value * 4096.0) as i32) as i16
}
