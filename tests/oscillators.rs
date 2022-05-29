//! Tests for the oscillators

mod modulation;
mod wav_writer;

use mi_plaits_dsp::dsp::oscillator::*;
use mi_plaits_dsp::dsp::SAMPLE_RATE;

const BLOCK_SIZE: usize = 24;

#[test]
fn formant_oscillator() {
    let carrier_frequency = 239.7;
    let formant_frequency = 105.0;
    let phase_shift = 0.75;
    let duration = 5.0;

    let mut osc = formant_oscillator::FormantOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f_carrier = carrier_frequency / SAMPLE_RATE;
    let f_formant = formant_frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = 1.0 + 4.0 * modulation::triangle(n, blocks, 1.0 / 8.0);
        osc.render(f_carrier, f_formant * modulation, phase_shift, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/formant.wav", &wav_data).ok();
}

#[test]
fn grainlet_oscillator() {
    let carrier_frequency = 80.0;
    let formant_frequency = 400.0;
    let carrier_bleed = 1.0;
    let duration = 5.0;

    let mut osc = grainlet_oscillator::GrainletOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f_carrier = carrier_frequency / SAMPLE_RATE;
    let f_formant = formant_frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation3 = modulation::triangle(n, blocks, 1.0 / 13.0);
        osc.render(f_carrier, f_formant, modulation3, carrier_bleed, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/grainlet.wav", &wav_data).ok();
}

#[test]
fn harmonic_oscillator() {
    let frequency = 110.0;
    let duration = 1.0;

    let mut osc = harmonic_oscillator::HarmonicOscillator::<8>::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;
    let mut amplitudes = [0.0; 8];
    amplitudes[1] = 0.1;

    for n in 0..blocks {
        amplitudes[0] = 0.5 - 0.5 * (n as f32) / (blocks as f32);
        amplitudes[5] = 0.1 * (n as f32) / (blocks as f32);
        osc.render(f0, &amplitudes, &mut out, 1);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/harmonic.wav", &wav_data).ok();
}

#[test]
fn oscillator_impulse_train() {
    let frequency = 112.0;
    let duration = 1.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(
            f,
            pw,
            None,
            &mut out,
            oscillator::OscillatorShape::ImpulseTrain,
            false,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/oscillator_impulse_train.wav", &wav_data).ok();
}

#[test]
fn oscillator_saw() {
    let frequency = 112.0;
    let duration = 1.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(
            f,
            pw,
            None,
            &mut out,
            oscillator::OscillatorShape::Saw,
            false,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/oscillator_saw.wav", &wav_data).ok();
}

#[test]
fn oscillator_triangle() {
    let frequency = 112.0;
    let duration = 1.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(
            f,
            pw,
            None,
            &mut out,
            oscillator::OscillatorShape::Triangle,
            false,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/oscillator_triangle.wav", &wav_data).ok();
}

#[test]
fn oscillator_slope() {
    let frequency = 112.0;
    let duration = 1.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(
            f,
            pw,
            None,
            &mut out,
            oscillator::OscillatorShape::Slope,
            false,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/oscillator_slope.wav", &wav_data).ok();
}

#[test]
fn oscillator_square() {
    let frequency = 112.0;
    let duration = 1.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(
            f,
            pw,
            None,
            &mut out,
            oscillator::OscillatorShape::Square,
            false,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/oscillator_square.wav", &wav_data).ok();
}

#[test]
fn oscillator_square_bright() {
    let frequency = 112.0;
    let duration = 1.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(
            f,
            pw,
            None,
            &mut out,
            oscillator::OscillatorShape::SquareBright,
            false,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/oscillator_square_bright.wav", &wav_data).ok();
}

#[test]
fn oscillator_square_dark() {
    let frequency = 112.0;
    let duration = 1.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(
            f,
            pw,
            None,
            &mut out,
            oscillator::OscillatorShape::SquareDark,
            false,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/oscillator_square_dark.wav", &wav_data).ok();
}

#[test]
fn oscillator_square_triangle() {
    let frequency = 112.0;
    let duration = 1.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(
            f,
            pw,
            None,
            &mut out,
            oscillator::OscillatorShape::SquareTriangle,
            false,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/oscillator_square_triangle.wav", &wav_data).ok();
}

#[test]
fn sine_oscillator() {
    let frequency = 440.0;
    let duration = 1.0;

    let mut osc = sine_oscillator::SineOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for _ in 0..blocks {
        osc.render(f, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/sine.wav", &wav_data).ok();
}

#[test]
fn fast_sine_oscillator() {
    let frequency = 440.0;
    let duration = 1.0;

    let mut osc = sine_oscillator::FastSineOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for _ in 0..blocks {
        osc.render(f, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/fastsine.wav", &wav_data).ok();
}

#[test]
fn string_synth_oscillator() {
    let frequency = 127.5;
    let registration: [f32; 7] = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let duration = 1.0;

    let mut osc = string_synth_oscillator::StringSynthOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for _ in 0..blocks {
        osc.render(f, &registration, 1.0, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/string_synth.wav", &wav_data).ok();
}

#[test]
fn variable_saw_oscillator() {
    let frequency = 220.0;
    let waveshape = 1.0;
    let duration = 1.0;

    let mut osc = variable_saw_oscillator::VariableSawOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(f, pw, waveshape, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/variable_saw.wav", &wav_data).ok();
}

#[test]
fn variable_shape_oscillator() {
    let master_frequency = 110.0;
    let frequency = 410.0;
    let waveshape = 1.0;
    let duration = 1.0;

    let mut osc = variable_shape_oscillator::VariableShapeOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let master_f = master_frequency / SAMPLE_RATE;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let pw = (n as f32) / (blocks as f32);
        osc.render(master_f, f, pw, waveshape, &mut out, true);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/variable_shape.wav", &wav_data).ok();
}

#[test]
fn vosim_oscillator() {
    let carrier_frequency = 105.0;
    let formant_frequency_1 = 1390.7;
    let formant_frequency_2 = 817.2;
    let duration = 5.0;

    let mut osc = vosim_oscillator::VosimOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let carrier_f = carrier_frequency / SAMPLE_RATE;
    let formant_f_1 = formant_frequency_1 / SAMPLE_RATE;
    let formant_f_2 = formant_frequency_2 / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::triangle(n, blocks, 1.0 / 8.0);
        osc.render(
            carrier_f,
            formant_f_1 * (1.0 + modulation),
            formant_f_2,
            modulation,
            &mut out,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/vosim.wav", &wav_data).ok();
}

#[test]
fn wavetable_oscillator() {
    // TODO: implement
}

#[test]
fn z_oscillator() {
    let carrier_frequency = 80.0;
    let formant_frequency = 250.7;
    let carrier_shape = 0.5;
    let duration = 5.0;

    let mut osc = z_oscillator::ZOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let carrier_f = carrier_frequency / SAMPLE_RATE;
    let formant_f = formant_frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::triangle(n, blocks, 1.0 / 7.0);
        let modulation_2 = modulation::triangle(n, blocks, 1.0 / 11.0);
        osc.render(
            carrier_f,
            formant_f * (1.0 + modulation * 8.0),
            modulation_2,
            carrier_shape,
            &mut out,
        );
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/z.wav", &wav_data).ok();
}
