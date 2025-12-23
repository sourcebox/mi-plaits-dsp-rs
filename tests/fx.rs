//! Tests for the effects

mod modulation;
mod wav_writer;

use mi_plaits_dsp::oscillator::sine_oscillator::SineOscillator;

use mi_plaits_dsp::fx::*;

const SAMPLE_RATE: f32 = 48000.0;
const BLOCK_SIZE: usize = 24;

#[test]
fn diffuser() {
    let amount = 1.0;
    let rt = 0.5;
    let duration = 1.0;

    let mut fx = diffuser::Diffuser::new();
    let mut in_out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    fx.init(SAMPLE_RATE);

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;

    for n in 0..blocks {
        in_out.fill(0.0);
        if n == 0 {
            in_out[0] = 1.0;
        }
        fx.process(amount, rt, &mut in_out);
        wav_data.extend_from_slice(&in_out);
    }

    wav_writer::write("fx/diffuser.wav", &wav_data).ok();
}

#[test]
fn ensemble() {
    let frequency = 220.0;
    let duration = 2.0;

    let mut osc = SineOscillator::new();
    let mut fx = ensemble::Ensemble::new();
    let mut left = [0.0; BLOCK_SIZE];
    let mut wav_data_left = Vec::new();
    let mut wav_data_right = Vec::new();
    osc.init();
    fx.init();
    fx.set_amount(0.5);
    fx.set_depth(0.5);

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for _ in 0..blocks {
        osc.render(f, &mut left);
        let mut right = left.clone();
        fx.process(&mut left, &mut right);
        wav_data_left.extend_from_slice(&left);
        wav_data_right.extend_from_slice(&right);
    }

    wav_writer::write("fx/ensemble_left.wav", &wav_data_left).ok();
    wav_writer::write("fx/ensemble_right.wav", &wav_data_right).ok();
}

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

#[test]
fn overdrive() {
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = SineOscillator::new();
    let mut fx = overdrive::Overdrive::new();
    let mut in_out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();
    fx.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        osc.render(f, &mut in_out);
        let drive = modulation::ramp_up(n, blocks);
        fx.process(drive, &mut in_out);
        wav_data.extend_from_slice(&in_out);
    }

    wav_writer::write("fx/overdrive.wav", &wav_data).ok();
}
