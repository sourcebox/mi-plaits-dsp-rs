//! Tests for six op engine

use mi_plaits_dsp::engine::*;
use mi_plaits_dsp::engine2::*;
use mi_plaits_dsp::resources::sysex::SYX_BANK_0;
const SAMPLE_RATE: f32 = 48000.0;
const A0_NORMALIZED: f32 = 55.0 / SAMPLE_RATE;

use crate::modulation;
use crate::wav_writer;

const BLOCK_SIZE: usize = 24;

#[test]
fn six_op_engine_harmonics() {
    let mut engine = six_op_engine::SixOpEngine::new(BLOCK_SIZE);
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    engine.init(SAMPLE_RATE);
    engine.load_syx_bank(&SYX_BANK_0);

    let duration = 2.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let mut already_enveloped = false;

    for n in 0..blocks {
        let parameters = EngineParameters {
            trigger: TriggerState::Unpatched,
            note: 48.0,
            timbre: 0.5,
            morph: 0.5,
            harmonics: modulation::ramp_up(n, blocks),
            accent: 1.0,
            a0_normalized: A0_NORMALIZED,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    wav_writer::write("engines/six_op/six_op_harmonics.wav", &wav_data).ok();
    wav_writer::write("engines/six_op/six_op_harmonics_aux.wav", &wav_data_aux).ok();
}

#[test]
fn six_op_engine_timbre() {
    let mut engine = six_op_engine::SixOpEngine::new(BLOCK_SIZE);
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    engine.init(SAMPLE_RATE);
    engine.load_syx_bank(&SYX_BANK_0);

    let duration = 2.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let mut already_enveloped = false;

    for n in 0..blocks {
        let parameters = EngineParameters {
            trigger: TriggerState::Unpatched,
            note: 48.0,
            timbre: modulation::ramp_up(n, blocks),
            morph: 0.5,
            harmonics: 0.5,
            accent: 1.0,
            a0_normalized: A0_NORMALIZED,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    wav_writer::write("engines/six_op/six_op_timbre.wav", &wav_data).ok();
    wav_writer::write("engines/six_op/six_op_timbre_aux.wav", &wav_data_aux).ok();
}

#[test]
fn six_op_engine_morph() {
    let mut engine = six_op_engine::SixOpEngine::new(BLOCK_SIZE);
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    engine.init(SAMPLE_RATE);
    engine.load_syx_bank(&SYX_BANK_0);

    let duration = 2.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let mut already_enveloped = false;

    for n in 0..blocks {
        let parameters = EngineParameters {
            trigger: TriggerState::Unpatched,
            note: 48.0,
            timbre: 0.5,
            morph: modulation::ramp_up(n, blocks),
            harmonics: 0.5,
            accent: 1.0,
            a0_normalized: A0_NORMALIZED,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    wav_writer::write("engines/six_op/six_op_morph.wav", &wav_data).ok();
    wav_writer::write("engines/six_op/six_op_morph_aux.wav", &wav_data_aux).ok();
}
