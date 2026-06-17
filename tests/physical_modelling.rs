//! Tests for the physical modelling

mod common;

use common::*;
use mi_plaits_dsp::physical_modelling::*;

const BLOCK_SIZE: usize = 24;

#[test]
fn modal_voice() {
    let frequency = 110.0;
    let accent = 1.0;
    let structure = 0.5;
    let brightness = 0.5;
    let damping = 0.5;
    let duration = 2.0;

    for sample_rate in SAMPLE_RATES {
        let mut model = modal_voice::ModalVoice::new();
        let mut out = [0.0; BLOCK_SIZE];
        let mut aux = [0.0; BLOCK_SIZE];
        let mut temp = [0.0; BLOCK_SIZE];
        let mut temp_2 = [0.0; BLOCK_SIZE];
        let mut wav_data = Vec::new();
        let mut wav_data_aux = Vec::new();
        model.init();

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;
        let f0 = frequency / sample_rate as f32;

        for n in 0..blocks {
            let trigger = n == 0;
            out.fill(0.0);
            aux.fill(0.0);
            temp.fill(0.0);
            temp_2.fill(0.0);
            model.render(
                false,
                trigger,
                accent,
                f0,
                structure,
                brightness,
                damping,
                &mut temp,
                &mut temp_2,
                &mut out,
                &mut aux,
            );
            wav_data.extend_from_slice(&out);
            wav_data_aux.extend_from_slice(&aux);
        }

        let filename = format!("physical_modelling/modal_voice/modal_voice_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();

        let filename_aux =
            format!("physical_modelling/modal_voice/modal_voice_aux_{sample_rate}.wav");
        write_wav(filename_aux, &wav_data_aux, sample_rate).ok();
    }
}

#[test]
fn resonator() {
    let frequency = 110.0;
    let position = 0.015;
    let resolution = 24;
    let structure = 0.5;
    let brightness = 0.5;
    let damping = 0.7;
    let duration = 2.0;

    for sample_rate in SAMPLE_RATES {
        let mut model = resonator::Resonator::new();
        let mut out = [0.0; BLOCK_SIZE];
        let mut wav_data = Vec::new();
        model.init(position, resolution);

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;
        let f0 = frequency / sample_rate as f32;

        for n in 0..blocks {
            let mut in_ = [0.0; BLOCK_SIZE];
            if n == 0 {
                in_[0] = 1.0;
            }
            out.fill(0.0);
            model.process(f0, structure, brightness, damping, &mut in_, &mut out);
            wav_data.extend_from_slice(&out);
        }

        let filename = format!("physical_modelling/resonator/resonator_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();
    }
}

#[test]
fn string() {
    let frequency = 110.0;
    let non_linearity_amount = 0.2;
    let brightness = 0.5;
    let damping = 0.7;
    let duration = 2.0;

    for sample_rate in SAMPLE_RATES {
        let mut model = string::String::new();
        let mut out = [0.0; BLOCK_SIZE];
        let mut wav_data = Vec::new();
        model.reset();

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;
        let f0 = frequency / sample_rate as f32;

        for n in 0..blocks {
            let mut in_ = [0.0; BLOCK_SIZE];
            if n == 0 {
                in_[0] = 1.0;
            }
            out.fill(0.0);
            model.process(
                f0,
                non_linearity_amount,
                brightness,
                damping,
                &mut in_,
                &mut out,
            );
            wav_data.extend_from_slice(&out);
        }

        let filename = format!("physical_modelling/string/string_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();
    }
}

#[test]
fn string_voice() {
    let frequency = 110.0;
    let accent = 1.0;
    let structure = 0.5;
    let brightness = 0.5;
    let damping = 0.7;
    let duration = 2.0;

    for sample_rate in SAMPLE_RATES {
        let mut model = string_voice::StringVoice::new();
        let mut out = [0.0; BLOCK_SIZE];
        let mut aux = [0.0; BLOCK_SIZE];
        let mut temp = [0.0; BLOCK_SIZE];
        let mut temp_2 = [0.0; BLOCK_SIZE];
        let mut wav_data = Vec::new();
        let mut wav_data_aux = Vec::new();
        model.init(sample_rate as f32);

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;
        let f0 = frequency / sample_rate as f32;

        for n in 0..blocks {
            let trigger = n == 0;
            out.fill(0.0);
            aux.fill(0.0);
            temp.fill(0.0);
            temp_2.fill(0.0);
            model.render(
                false,
                trigger,
                accent,
                f0,
                structure,
                brightness,
                damping,
                &mut temp,
                &mut temp_2,
                &mut out,
                &mut aux,
            );
            wav_data.extend_from_slice(&out);
            wav_data_aux.extend_from_slice(&aux);
        }

        let filename = format!("physical_modelling/string_voice/string_voice_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();

        let filename_aux =
            format!("physical_modelling/string_voice/string_voice_aux_{sample_rate}.wav");
        write_wav(filename_aux, &wav_data_aux, sample_rate).ok();
    }
}
