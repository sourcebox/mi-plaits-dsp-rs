//! Tests for additive engine

use mi_plaits_dsp::engine::*;
use mi_plaits_dsp::SAMPLE_RATE;

use crate::modulation;
use crate::wav_writer;

const BLOCK_SIZE: usize = 24;

#[test]
fn additive_engine_harmonics() {
    let mut engine = additive_engine::AdditiveEngine::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    engine.init();

    let duration = 2.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let mut already_enveloped = false;

    for n in 0..blocks {
        let parameters = EngineParameters {
            trigger: if n == 0 {
                TriggerState::RisingEdge
            } else {
                TriggerState::Low
            },
            note: 48.0,
            timbre: 0.2,
            morph: 0.5,
            harmonics: modulation::ramp_up(n, blocks),
            accent: 1.0,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    wav_writer::write("engines/additive/additive_harmonics.wav", &wav_data).ok();
    wav_writer::write("engines/additive/additive_harmonics_aux.wav", &wav_data_aux).ok();
}

#[test]
fn additive_engine_timbre() {
    let mut engine = additive_engine::AdditiveEngine::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    engine.init();

    let duration = 2.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let mut already_enveloped = false;

    for n in 0..blocks {
        let parameters = EngineParameters {
            trigger: if n == 0 {
                TriggerState::RisingEdge
            } else {
                TriggerState::Low
            },
            note: 48.0,
            timbre: modulation::ramp_up(n, blocks),
            morph: 0.5,
            harmonics: 0.5,
            accent: 1.0,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    wav_writer::write("engines/additive/additive_timbre.wav", &wav_data).ok();
    wav_writer::write("engines/additive/additive_timbre_aux.wav", &wav_data_aux).ok();
}

#[test]
fn additive_engine_morph() {
    let mut engine = additive_engine::AdditiveEngine::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    engine.init();

    let duration = 2.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let mut already_enveloped = false;

    for n in 0..blocks {
        let parameters = EngineParameters {
            trigger: if n == 0 {
                TriggerState::RisingEdge
            } else {
                TriggerState::Low
            },
            note: 48.0,
            timbre: 0.2,
            morph: modulation::ramp_up(n, blocks),
            harmonics: 0.5,
            accent: 1.0,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    wav_writer::write("engines/additive/additive_morph.wav", &wav_data).ok();
    wav_writer::write("engines/additive/additive_morph_aux.wav", &wav_data_aux).ok();
}
