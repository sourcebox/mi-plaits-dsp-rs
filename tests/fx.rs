//! Tests for the effects

mod common;

use common::*;
use mi_plaits_dsp::fx::*;
use mi_plaits_dsp::oscillator::sine_oscillator::SineOscillator;

const BLOCK_SIZE: usize = 24;
const RESET_PROBE_BLOCKS: usize = 400;

#[test]
fn diffuser_reset_clears_delay_lines() {
    let mut fx = diffuser::Diffuser::new();
    fx.init(48_000.0);

    let mut in_out = [1.0; BLOCK_SIZE];
    fx.process(1.0, 0.5, &mut in_out);

    // Clearing the delay lines must not clear the persistent LP or LFO state,
    // so use a clone with an explicit clear.
    let mut expected = fx.clone();
    fx.reset();
    expected.clear();

    for block in 0..RESET_PROBE_BLOCKS {
        in_out.fill(0.0);
        let mut expected_out = in_out;
        fx.process(1.0, 0.5, &mut in_out);
        expected.process(1.0, 0.5, &mut expected_out);
        assert_eq!(
            in_out, expected_out,
            "diffuser reset did not clear its delay lines in probe block {block}"
        );
    }
}

#[test]
fn ensemble_reset_clears_delay_lines() {
    let mut fx = ensemble::Ensemble::new();
    fx.init();
    fx.set_amount(1.0);
    fx.set_depth(0.5);

    let mut left = [1.0; BLOCK_SIZE];
    let mut right = [1.0; BLOCK_SIZE];
    fx.process(&mut left, &mut right);
    fx.reset();

    // A single block is not long enough for stale samples to emerge from the
    // chorus lines, so observe several complete delay lengths.
    for block in 0..RESET_PROBE_BLOCKS {
        left.fill(0.0);
        right.fill(0.0);
        fx.process(&mut left, &mut right);
        assert!(
            left.iter().all(|sample| *sample == 0.0) && right.iter().all(|sample| *sample == 0.0),
            "ensemble produced a stale sample after reset in probe block {block}"
        );
    }
}

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
            let mut right = left;
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
