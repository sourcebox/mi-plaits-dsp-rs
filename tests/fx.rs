//! Tests for the effects

mod common;

use common::*;
use mi_plaits_dsp::fx::*;
use mi_plaits_dsp::oscillator::sine_oscillator::SineOscillator;

const BLOCK_SIZE: usize = 24;

#[test]
fn diffuser() {
    let amount = 1.0;
    let rt = 0.5;
    let duration = 1.0;

    for sample_rate in SAMPLE_RATES {
        let mut fx = diffuser::Diffuser::new();
        let mut in_out = [0.0; BLOCK_SIZE];
        let mut wav_data = Vec::new();
        fx.init(sample_rate as f32);

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;

        for n in 0..blocks {
            in_out.fill(0.0);
            if n == 0 {
                in_out[0] = 1.0;
            }
            fx.process(amount, rt, &mut in_out);
            wav_data.extend_from_slice(&in_out);
        }

        let filename = format!("fx/diffuser/diffuser_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();
    }
}

#[test]
fn ensemble() {
    let frequency = 220.0;
    let duration = 2.0;

    for sample_rate in SAMPLE_RATES {
        let mut osc = SineOscillator::new();
        let mut fx = ensemble::Ensemble::new();
        let mut left = [0.0; BLOCK_SIZE];
        let mut wav_data_left = Vec::new();
        let mut wav_data_right = Vec::new();
        osc.init();
        fx.init();
        fx.set_amount(0.5);
        fx.set_depth(0.5);

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;
        let f = frequency / sample_rate as f32;

        for _ in 0..blocks {
            osc.render(f, &mut left);
            let mut right = left.clone();
            fx.process(&mut left, &mut right);
            wav_data_left.extend_from_slice(&left);
            wav_data_right.extend_from_slice(&right);
        }

        let filename_left = format!("fx/ensemble/ensemble_left_{sample_rate}.wav");
        write_wav(filename_left, &wav_data_left, sample_rate).ok();

        let filename_right = format!("fx/ensemble/ensemble_right_{sample_rate}.wav");
        write_wav(filename_right, &wav_data_right, sample_rate).ok();
    }
}

#[test]
fn sample_rate_reducer() {
    let frequency = 110.0;
    let duration = 2.0;

    for sample_rate in SAMPLE_RATES {
        let mut osc = SineOscillator::new();
        let mut fx = sample_rate_reducer::SampleRateReducer::new();
        let mut in_out = [0.0; BLOCK_SIZE];
        let mut wav_data = Vec::new();
        osc.init();
        fx.init();

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;
        let f = frequency / sample_rate as f32;

        for n in 0..blocks {
            osc.render(f, &mut in_out);
            let fx_f = mod_ramp_up(n, blocks) * 0.1;
            fx.process(fx_f, &mut in_out, true);
            wav_data.extend_from_slice(&in_out);
        }

        let filename = format!("fx/sample_rate_reducer/sample_rate_reducer_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();
    }
}

#[test]
fn overdrive() {
    let frequency = 110.0;
    let duration = 2.0;

    for sample_rate in SAMPLE_RATES {
        let mut osc = SineOscillator::new();
        let mut fx = overdrive::Overdrive::new();
        let mut in_out = [0.0; BLOCK_SIZE];
        let mut wav_data = Vec::new();
        osc.init();
        fx.init();

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;
        let f = frequency / sample_rate as f32;

        for n in 0..blocks {
            osc.render(f, &mut in_out);
            let drive = mod_ramp_up(n, blocks);
            fx.process(drive, &mut in_out);
            wav_data.extend_from_slice(&in_out);
        }

        let filename = format!("fx/overdrive/overdrive_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();
    }
}
