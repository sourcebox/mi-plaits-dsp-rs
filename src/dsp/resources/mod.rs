//! Resources definitions.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

pub mod fm;
pub mod fold;
pub mod lpc;
pub mod sine;
pub mod stiffness;
pub mod svf;
pub mod waves;
pub mod waveshape;

pub const LOOKUP_TABLE_TABLE: [&[f32]; 6] = [
    sine::LUT_SINE.as_slice(),
    fm::LUT_FM_FREQUENCY_QUANTIZER.as_slice(),
    fold::LUT_FOLD.as_slice(),
    fold::LUT_FOLD_2.as_slice(),
    stiffness::LUT_STIFFNESS.as_slice(),
    svf::LUT_SVF_SHIFT.as_slice(),
];
