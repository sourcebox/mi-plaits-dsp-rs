//! Tests for the voice module.

mod wav_writer;

use mi_plaits_dsp::dsp::voice::{Modulations, Patch, Voice, NUM_ENGINES};
use mi_plaits_dsp::dsp::SAMPLE_RATE;

const BLOCK_SIZE: usize = 24;

#[test]
fn all_engines() {
    let mut voice = Voice::new(BLOCK_SIZE);
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    voice.init();

    let duration = 1.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;

    let mut patch = Patch {
        note: 48.0,
        harmonics: 0.5,
        timbre: 0.5,
        morph: 0.5,
        frequency_modulation_amount: 0.0,
        timbre_modulation_amount: 0.5,
        morph_modulation_amount: 0.5,
        engine: 0,
        decay: 0.5,
        lpg_colour: 0.5,
    };

    let modulations = Modulations {
        engine: 0.0,
        note: 0.0,
        frequency: 0.0,
        harmonics: 0.0,
        timbre: 0.0,
        morph: 0.0,
        trigger: 0.0,
        level: 0.0,
        frequency_patched: false,
        timbre_patched: false,
        morph_patched: false,
        trigger_patched: false,
        level_patched: false,
    };

    for engine in 0..NUM_ENGINES {
        patch.engine = engine;

        for _ in 0..blocks {
            voice.render(&patch, &modulations, &mut out, &mut aux);
            wav_data.extend_from_slice(&out);
            wav_data_aux.extend_from_slice(&aux);
        }
    }

    wav_writer::write("voice/all_engines.wav", &wav_data).ok();
    wav_writer::write("voice/all_engines_aux.wav", &wav_data_aux).ok();
}

#[test]
fn all_engines_trigger() {
    let mut voice = Voice::new(BLOCK_SIZE);
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();

    voice.init();

    let duration = 1.0;
    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;

    let mut patch = Patch {
        note: 48.0,
        harmonics: 0.5,
        timbre: 0.5,
        morph: 0.5,
        frequency_modulation_amount: 0.0,
        timbre_modulation_amount: 0.5,
        morph_modulation_amount: 0.5,
        engine: 0,
        decay: 0.5,
        lpg_colour: 0.5,
    };

    let mut modulations = Modulations {
        engine: 0.0,
        note: 0.0,
        frequency: 0.0,
        harmonics: 0.0,
        timbre: 0.0,
        morph: 0.0,
        trigger: 0.0,
        level: 0.0,
        frequency_patched: false,
        timbre_patched: false,
        morph_patched: false,
        trigger_patched: true,
        level_patched: false,
    };

    for engine in 0..NUM_ENGINES {
        patch.engine = engine;

        for n in 0..blocks {
            modulations.trigger = if n == 0 { 1.0 } else { 0.0 };
            voice.render(&patch, &modulations, &mut out, &mut aux);
            wav_data.extend_from_slice(&out);
            wav_data_aux.extend_from_slice(&aux);
        }
    }

    wav_writer::write("voice/all_engines_trigger.wav", &wav_data).ok();
    wav_writer::write("voice/all_engines_trigger_aux.wav", &wav_data_aux).ok();
}
