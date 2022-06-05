//! Tests for the effects

mod modulation;
mod wav_writer;

use mi_plaits_dsp::dsp::oscillator::sine_oscillator::SineOscillator;

use mi_plaits_dsp::dsp::fx::*;
use mi_plaits_dsp::dsp::SAMPLE_RATE;

const BLOCK_SIZE: usize = 24;

#[test]
fn sample_rate_reducer() {
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = SineOscillator::new();
    let mut fx = sample_rate_reducer::SampleRateReducer::new();
    let mut in_out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();
    fx.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        osc.render(f, &mut in_out);
        let fx_f = modulation::ramp_up(n, blocks) * 0.1;
        fx.process(fx_f, &mut in_out, true);
        wav_data.extend_from_slice(&in_out);
    }

    wav_writer::write("fx/sample_rate_reducer.wav", &wav_data).ok();
}
