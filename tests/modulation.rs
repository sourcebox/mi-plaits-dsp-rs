//! Modulation sources

/// Returns a triangle wave in range -1.0..1.0
pub fn triangle(block_no: usize, block_count: usize, periods: f32) -> f32 {
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
pub fn ramp_up(block_no: usize, block_count: usize) -> f32 {
    let phase = block_no as f32 / block_count as f32;

    phase
}
