//! Tests for modal engine

use mi_plaits_dsp::engine::*;
const SAMPLE_RATE: f32 = 48000.0;
const A0_NORMALIZED: f32 = 55.0 / SAMPLE_RATE;

use crate::common::*;

const BLOCK_SIZE: usize = 24;

#[test]
fn modal_engine_harmonics() {
    let mut engine = modal_engine::ModalEngine::new(BLOCK_SIZE);
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    engine.init(SAMPLE_RATE);

    let duration = 2.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let mut already_enveloped = false;

    for n in 0..blocks {
        let parameters = EngineParameters {
            trigger: if n % (blocks / 5) == 0 {
                TriggerState::RisingEdge
            } else {
                TriggerState::Low
            },
            note: 48.0,
            timbre: 0.5,
            morph: 0.5,
            harmonics: mod_ramp_up(n, blocks),
            accent: 1.0,
            a0_normalized: A0_NORMALIZED,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    write_wav(
        "engines/modal/modal_harmonics.wav",
        &wav_data,
        SAMPLE_RATE as u32,
    )
    .ok();
    write_wav(
        "engines/modal/modal_harmonics_aux.wav",
        &wav_data_aux,
        SAMPLE_RATE as u32,
    )
    .ok();
}

#[test]
fn modal_engine_timbre() {
    let mut engine = modal_engine::ModalEngine::new(BLOCK_SIZE);
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    engine.init(SAMPLE_RATE);

    let duration = 2.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let mut already_enveloped = false;

    for n in 0..blocks {
        let parameters = EngineParameters {
            trigger: if n % (blocks / 5) == 0 {
                TriggerState::RisingEdge
            } else {
                TriggerState::Low
            },
            note: 48.0,
            timbre: mod_ramp_up(n, blocks),
            morph: 0.5,
            harmonics: 0.5,
            accent: 1.0,
            a0_normalized: A0_NORMALIZED,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    write_wav(
        "engines/modal/modal_timbre.wav",
        &wav_data,
        SAMPLE_RATE as u32,
    )
    .ok();
    write_wav(
        "engines/modal/modal_timbre_aux.wav",
        &wav_data_aux,
        SAMPLE_RATE as u32,
    )
    .ok();
}

#[test]
fn modal_engine_morph() {
    let mut engine = modal_engine::ModalEngine::new(BLOCK_SIZE);
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    engine.init(SAMPLE_RATE);

    let duration = 2.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let mut already_enveloped = false;

    for n in 0..blocks {
        let parameters = EngineParameters {
            trigger: if n % (blocks / 5) == 0 {
                TriggerState::RisingEdge
            } else {
                TriggerState::Low
            },
            note: 48.0,
            timbre: 0.5,
            morph: mod_ramp_up(n, blocks),
            harmonics: 0.5,
            accent: 1.0,
            a0_normalized: A0_NORMALIZED,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    write_wav(
        "engines/modal/modal_morph.wav",
        &wav_data,
        SAMPLE_RATE as u32,
    )
    .ok();
    write_wav(
        "engines/modal/modal_morph_aux.wav",
        &wav_data_aux,
        SAMPLE_RATE as u32,
    )
    .ok();
}
