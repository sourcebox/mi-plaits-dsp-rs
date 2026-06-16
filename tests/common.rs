//! Common helpers for all tests.

#![allow(unused)]

use std::path::Path;

use hound::*;

/// Writes sample data as WAV file in 32-bit float format.
pub fn write_wav(
    filename: impl AsRef<std::path::Path> + core::fmt::Display,
    samples: &[f32],
    sample_rate: u32,
) -> std::io::Result<()> {
    let path = format!("out/{filename}");
    let path = Path::new(path.as_str());

    // Create parent directories to the path if they don't exist.
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent).ok();

    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(path, spec).unwrap();

    for sample in samples {
        writer.write_sample(*sample).unwrap();
        writer.write_sample(*sample).unwrap();
    }

    Ok(())
}

/// Returns a triangle wave in range -1.0..1.0
pub fn mod_triangle(block_no: usize, block_count: usize, periods: f32) -> f32 {
    let mut phase = block_no as f32 / block_count as f32 * periods;

    while phase > 1.0 {
        phase -= 1.0
    }

    let waveform = if phase < 0.25 {
        phase * 4.0
    } else if phase < 0.5 {
        (0.5 - phase) * 4.0
    } else if phase < 0.75 {
        -(phase - 0.5) * 4.0
    } else {
        -(1.0 - phase) * 4.0
    };

    waveform
}

/// Returns a ramp in range 0.0..1.0
pub fn mod_ramp_up(block_no: usize, block_count: usize) -> f32 {
    let phase = block_no as f32 / block_count as f32;

    phase
}
