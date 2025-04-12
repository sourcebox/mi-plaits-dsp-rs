//! Effects used by different engines.

pub mod diffuser;
pub mod ensemble;
pub mod low_pass_gate;
pub mod overdrive;
pub mod sample_rate_reducer;

use core::marker::PhantomData;

use crate::utils::clip_16;
use crate::utils::cosine_oscillator::{CosineOscillator, CosineOscillatorMode};
use crate::utils::delay_line::DelayLine;
use num_traits::{FromPrimitive, Num, Signed, ToPrimitive};

pub trait DataType {}

#[derive(Debug, Default)]
pub struct DataFormat12Bit;

#[derive(Debug, Default)]
pub struct DataFormat32Bit;

impl DataType for DataFormat12Bit {}
impl DataType for DataFormat32Bit {}

#[derive(Debug, Default)]
pub struct FxEngine<const SIZE: usize, DT>
where
    DT: DataType,
{
    lfo: CosineOscillator,
    write_ptr: i32,
    _data_type: PhantomData<DT>,
}

impl<const SIZE: usize, DT> FxEngine<SIZE, DT>
where
    DT: DataType + Default,
{
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

    pub fn start(&mut self, c: &mut FxContext<DT>) {
        self.write_ptr -= 1;

        if self.write_ptr < 0 {
            self.write_ptr += SIZE as i32;
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
pub struct FxContext<DT>
where
    DT: DataType,
{
    accumulator: f32,
    previous_read: f32,
    lfo_value: f32,
    _data_type: PhantomData<DT>,
}

impl<DT> FxContext<DT>
where
    DT: DataType + Default,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&mut self, value: f32) {
        self.accumulator += value;
    }

    pub fn read_with_scale(&mut self, value: f32, scale: f32) {
        self.accumulator += value * scale;
    }

    pub fn write(&mut self, value: &mut f32) {
        *value = self.accumulator;
    }

    pub fn write_with_scale(&mut self, value: &mut f32, scale: f32) {
        *value = self.accumulator;
        self.accumulator *= scale;
    }
}

impl FxContext<DataFormat12Bit> {
    pub fn write_line<T, const SIZE: usize>(&mut self, line: &mut DelayLine<T, SIZE>, scale: f32)
    where
        T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
    {
        let w = compress_12bit(self.accumulator);
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
        let r_f = decompress_12bit(r.to_i16().unwrap_or_default());
        self.previous_read = r_f;
        self.accumulator += r_f * scale;
    }

    pub fn lp(&mut self, state: &mut f32, coefficient: f32) {
        *state += coefficient * (self.accumulator - *state);
        self.accumulator = *state;
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
        let x = decompress_12bit(
            line.read_with_delay_frac((line.max_delay()) as f32 + offset)
                .to_i16()
                .unwrap_or_default(),
        );
        self.previous_read = x;
        self.accumulator += x * scale;
    }
}

impl FxContext<DataFormat32Bit> {
    pub fn write_line<T, const SIZE: usize>(&mut self, line: &mut DelayLine<T, SIZE>, scale: f32)
    where
        T: Copy + Default + Num + Signed + FromPrimitive + ToPrimitive,
    {
        let w = self.accumulator;
        line.write(T::from_f32(w).unwrap_or_default());
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
        let r_f = r.to_f32().unwrap_or_default();
        self.previous_read = r_f;
        self.accumulator += r_f * scale;
    }

    pub fn lp(&mut self, state: &mut f32, coefficient: f32) {
        *state += coefficient * (self.accumulator - *state);
        self.accumulator = *state;
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
        let x = line
            .read_with_delay_frac((line.max_delay()) as f32 + offset)
            .to_f32()
            .unwrap_or_default();
        self.previous_read = x;
        self.accumulator += x * scale;
    }
}

fn decompress_12bit(value: i16) -> f32 {
    value as f32 / 4096.0
}

fn compress_12bit(value: f32) -> i16 {
    clip_16((value * 4096.0) as i32) as i16
}
