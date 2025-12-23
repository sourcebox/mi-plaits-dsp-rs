//! Tests for virtual analog vcf engine

use mi_plaits_dsp::engine::*;
use mi_plaits_dsp::engine2::*;
const SAMPLE_RATE: f32 = 48000.0;
const A0_NORMALIZED: f32 = 55.0 / SAMPLE_RATE;

use crate::modulation;
use crate::wav_writer;

const BLOCK_SIZE: usize = 24;

#[test]
fn virtual_analog_vcf_engine_harmonics() {
    let mut engine = virtual_analog_vcf_engine::VirtualAnalogVcfEngine::new();
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
            trigger: if n == 0 {
                TriggerState::RisingEdge
            } else {
                TriggerState::Low
            },
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

    wav_writer::write(
        "engines/virtual_analog_vcf/virtual_analog_vcf_harmonics.wav",
        &wav_data,
    )
    .ok();
    wav_writer::write(
        "engines/virtual_analog_vcf/virtual_analog_vcf_harmonics_aux.wav",
        &wav_data_aux,
    )
    .ok();
}

#[test]
fn virtual_analog_vcf_engine_timbre() {
    let mut engine = virtual_analog_vcf_engine::VirtualAnalogVcfEngine::new();
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
            a0_normalized: A0_NORMALIZED,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    wav_writer::write(
        "engines/virtual_analog_vcf/virtual_analog_vcf_timbre.wav",
        &wav_data,
    )
    .ok();
    wav_writer::write(
        "engines/virtual_analog_vcf/virtual_analog_vcf_timbre_aux.wav",
        &wav_data_aux,
    )
    .ok();
}

#[test]
fn virtual_analog_vcf_engine_morph() {
    let mut engine = virtual_analog_vcf_engine::VirtualAnalogVcfEngine::new();
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
            trigger: if n == 0 {
                TriggerState::RisingEdge
            } else {
                TriggerState::Low
            },
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

    wav_writer::write(
        "engines/virtual_analog_vcf/virtual_analog_vcf_morph.wav",
        &wav_data,
    )
    .ok();
    wav_writer::write(
        "engines/virtual_analog_vcf/virtual_analog_vcf_morph_aux.wav",
        &wav_data_aux,
    )
    .ok();
}
