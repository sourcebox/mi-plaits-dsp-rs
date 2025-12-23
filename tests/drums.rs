//! Tests for the drums

mod wav_writer;

use mi_plaits_dsp::drums::*;

const SAMPLE_RATE: f32 = 48000.0;
const BLOCK_SIZE: usize = 24;

#[test]
fn analog_bass_drum() {
    let frequency = 50.0;
    let accent = 1.0;
    let tone = 1.0;
    let decay = 0.5;
    let attack_fm_amount = 0.0;
    let self_fm_amount = 0.0;
    let duration = 0.5;

    let mut drum = analog_bass_drum::AnalogBassDrum::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    drum.init(SAMPLE_RATE);

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let trigger = n == 0;
        drum.render(
            false,
            trigger,
            accent,
            f0,
            tone,
            decay,
            attack_fm_amount,
            self_fm_amount,
            &mut out,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("drums/analog_bass_drum.wav", &wav_data).ok();
}

#[test]
fn analog_snare_drum() {
    let frequency = 200.0;
    let accent = 1.0;
    let tone = 1.0;
    let decay = 0.5;
    let snappy = 0.2;
    let duration = 0.5;

    let mut drum = analog_snare_drum::AnalogSnareDrum::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    drum.init(SAMPLE_RATE);

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let trigger = n == 0;
        drum.render(false, trigger, accent, f0, tone, decay, snappy, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("drums/analog_snare_drum.wav", &wav_data).ok();
}

#[test]
fn hihat() {
    let frequency = 440.0;
    let accent = 1.0;
    let tone = 1.0;
    let decay = 0.5;
    let noisiness = 0.0;
    let duration = 0.5;

    let mut drum = hihat::Hihat::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut temp_1 = [0.0; BLOCK_SIZE];
    let mut temp_2 = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    drum.init(SAMPLE_RATE);

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let trigger = n == 0;
        drum.render(
            false,
            trigger,
            accent,
            f0,
            tone,
            decay,
            noisiness,
            &mut out,
            &mut temp_1,
            &mut temp_2,
            hihat::NoiseType::RingMod,
            hihat::VcaType::Swing,
            false,
            false,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("drums/hihat.wav", &wav_data).ok();
}

#[test]
fn synthetic_bass_drum() {
    let frequency = 50.0;
    let accent = 1.0;
    let tone = 1.0;
    let decay = 0.5;
    let dirtiness = 0.5;
    let fm_envelope_amount = 0.5;
    let fm_envelope_decay = 0.5;
    let duration = 0.5;

    let mut drum = synthetic_bass_drum::SyntheticBassDrum::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    drum.init(SAMPLE_RATE);

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let trigger = n == 0;
        drum.render(
            false,
            trigger,
            accent,
            f0,
            tone,
            decay,
            dirtiness,
            fm_envelope_amount,
            fm_envelope_decay,
            &mut out,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("drums/synthetic_bass_drum.wav", &wav_data).ok();
}

#[test]
fn synthetic_snare_drum() {
    let frequency = 200.0;
    let accent = 1.0;
    let fm_amount = 0.5;
    let decay = 0.3;
    let snappy = 0.2;
    let duration = 0.5;

    let mut drum = synthetic_snare_drum::SyntheticSnareDrum::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    drum.init(SAMPLE_RATE);

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let trigger = n == 0;
        drum.render(
            false, trigger, accent, f0, fm_amount, decay, snappy, &mut out,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("drums/synthetic_snare_drum.wav", &wav_data).ok();
}
