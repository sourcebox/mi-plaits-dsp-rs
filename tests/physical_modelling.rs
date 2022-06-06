//! Tests for the physical modelling

mod wav_writer;

use mi_plaits_dsp::dsp::physical_modelling::*;
use mi_plaits_dsp::dsp::SAMPLE_RATE;

const BLOCK_SIZE: usize = 24;

#[test]
fn delay_line() {
    // TODO: implement
}

#[test]
fn modal_voice() {
    let frequency = 110.0;
    let accent = 0.5;
    let structure = 0.1;
    let brightness = 0.1;
    let damping = 0.5;
    let duration = 1.0;

    let mut model = modal_voice::ModalVoice::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut temp = [0.0; BLOCK_SIZE];
    let mut temp_2 = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();
    model.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

    for _ in 0..blocks {
        model.render(
            true,
            true,
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

    wav_writer::write("physical_modelling/modal_voice.wav", &wav_data).ok();
    wav_writer::write("physical_modelling/modal_voice_aux.wav", &wav_data_aux).ok();
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

    let mut model = resonator::Resonator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    model.init(position, resolution);

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let mut in_ = [0.0; BLOCK_SIZE];
        if n == 0 {
            in_[0] = 1.0;
        }
        out.fill(0.0);
        model.process(f0, structure, brightness, damping, &mut in_, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("physical_modelling/resonator.wav", &wav_data).ok();
}

#[test]
fn string() {
    let frequency = 110.0;
    let non_linearity_amount = 0.2;
    let brightness = 0.5;
    let damping = 0.7;
    let duration = 2.0;

    let mut model = string::String::new(&std::alloc::System);
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    model.reset();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

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

    wav_writer::write("physical_modelling/string.wav", &wav_data).ok();
}

#[test]
fn string_voice() {
    let frequency = 110.0;
    let accent = 1.0;
    let structure = 0.5;
    let brightness = 0.5;
    let damping = 0.7;
    let duration = 2.0;

    let mut model = string_voice::StringVoice::new(&std::alloc::System);
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut temp = [0.0; BLOCK_SIZE];
    let mut temp_2 = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();
    model.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

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

    wav_writer::write("physical_modelling/string_voice.wav", &wav_data).ok();
    wav_writer::write("physical_modelling/string_voice_aux.wav", &wav_data_aux).ok();
}
