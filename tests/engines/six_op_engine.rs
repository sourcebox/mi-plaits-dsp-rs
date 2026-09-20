//! Tests for six op engine

use mi_plaits_dsp::engine::*;
use mi_plaits_dsp::engine2::*;
use mi_plaits_dsp::resources::sysex::SYX_BANK_0;
const SAMPLE_RATE: f32 = 48000.0;
const A0_NORMALIZED: f32 = 55.0 / SAMPLE_RATE;

use crate::common::*;

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
            harmonics: mod_ramp_up(n, blocks),
            accent: 1.0,
            a0_normalized: A0_NORMALIZED,
        };

        engine.render(&parameters, &mut out, &mut aux, &mut already_enveloped);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    write_wav(
        "engines/six_op/six_op_harmonics.wav",
        &wav_data,
        SAMPLE_RATE as u32,
    )
    .ok();
    write_wav(
        "engines/six_op/six_op_harmonics_aux.wav",
        &wav_data_aux,
        SAMPLE_RATE as u32,
    )
    .ok();
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
        "engines/six_op/six_op_timbre.wav",
        &wav_data,
        SAMPLE_RATE as u32,
    )
    .ok();
    write_wav(
        "engines/six_op/six_op_timbre_aux.wav",
        &wav_data_aux,
        SAMPLE_RATE as u32,
    )
    .ok();
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
        "engines/six_op/six_op_morph.wav",
        &wav_data,
        SAMPLE_RATE as u32,
    )
    .ok();
    write_wav(
        "engines/six_op/six_op_morph_aux.wav",
        &wav_data_aux,
        SAMPLE_RATE as u32,
    )
    .ok();
}

/// Renders one block with bank 0's "CS 80" patch, which has delayed,
/// non-key-synced LFO pitch modulation.
fn render_cs80_block(
    engine: &mut six_op_engine::SixOpEngine<'_>,
    trigger: TriggerState,
    out: &mut [f32; BLOCK_SIZE],
) {
    let parameters = EngineParameters {
        trigger,
        note: 48.0,
        timbre: 0.5,
        morph: 0.5,
        harmonics: 0.5,
        accent: 1.0,
        a0_normalized: A0_NORMALIZED,
    };
    let mut aux = [0.0; BLOCK_SIZE];
    let mut already_enveloped = false;

    engine.render(&parameters, out, &mut aux, &mut already_enveloped);
}

/// An unloaded voice's LFO must remain idle so its first note does not inherit
/// invalid phase and produce runaway pitch modulation after the LFO delay.
#[test]
fn six_op_engine_second_voice_starts_with_settled_lfo() {
    const FIRST_NOTE_BLOCKS: usize = 2500;
    const SECOND_NOTE_BLOCKS: usize = 3000;
    const MEASURED_BLOCKS: usize = 1000;
    const MAX_STEP: f32 = 0.25;

    let mut engine = six_op_engine::SixOpEngine::new(BLOCK_SIZE);
    let mut out = [0.0; BLOCK_SIZE];

    engine.init(SAMPLE_RATE);
    engine.load_syx_bank(&SYX_BANK_0);

    // Hold the first note while the second voice remains unloaded.
    render_cs80_block(&mut engine, TriggerState::RisingEdge, &mut out);
    for _ in 1..FIRST_NOTE_BLOCKS {
        render_cs80_block(&mut engine, TriggerState::High, &mut out);
    }
    render_cs80_block(&mut engine, TriggerState::Low, &mut out);

    // Rotate to the second voice and render past its LFO delay.
    render_cs80_block(&mut engine, TriggerState::RisingEdge, &mut out);
    let mut previous = out[BLOCK_SIZE - 1];
    let mut max_step: f32 = 0.0;
    for block in 1..SECOND_NOTE_BLOCKS {
        render_cs80_block(&mut engine, TriggerState::High, &mut out);
        for sample in out {
            if block >= SECOND_NOTE_BLOCKS - MEASURED_BLOCKS {
                max_step = max_step.max((sample - previous).abs());
            }
            previous = sample;
        }
    }

    assert!(
        max_step < MAX_STEP,
        "second voice output has a sample step of {max_step}, expected below {MAX_STEP}"
    );
}
