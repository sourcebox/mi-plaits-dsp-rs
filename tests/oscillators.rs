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
    let duration = 2.0;

    let mut osc = formant_oscillator::FormantOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f_carrier = carrier_frequency / SAMPLE_RATE;
    let f_formant = formant_frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
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
    let duration = 2.0;

    let mut osc = grainlet_oscillator::GrainletOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f_carrier = carrier_frequency / SAMPLE_RATE;
    let f_formant = formant_frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        osc.render(f_carrier, f_formant, modulation, carrier_bleed, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/grainlet.wav", &wav_data).ok();
}

#[test]
fn harmonic_oscillator() {
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = harmonic_oscillator::HarmonicOscillator::<8>::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;
    let mut amplitudes = [0.0; 8];
    amplitudes[1] = 0.1;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        amplitudes[0] = 0.5;
        amplitudes[5] = modulation;
        osc.render(f0, &amplitudes, &mut out, 1);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/harmonic.wav", &wav_data).ok();
}

#[test]
fn nes_triangle_oscillator() {
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = nes_triangle_oscillator::NesTriangleOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

    for _ in 0..blocks {
        osc.render(f0, &mut out, 5);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/nes_triangle.wav", &wav_data).ok();
}

#[test]
fn oscillator_impulse_train() {
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
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
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
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
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
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
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
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
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
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
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
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
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
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
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = oscillator::Oscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
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
    let frequency = 110.0;
    let duration = 2.0;

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
    let frequency = 110.0;
    let duration = 2.0;

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
    let frequency = 55.0;
    let registration: [f32; 7] = [1.0, 0.0, 0.5, 0.0, 0.2, 0.0, 0.5];
    let duration = 2.0;

    let mut osc = string_synth_oscillator::StringSynthOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for _ in 0..blocks {
        out.fill(0.0);
        osc.render(f, &registration, 1.0, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/string_synth.wav", &wav_data).ok();
}

#[test]
fn super_square_oscillator() {
    let frequency = 110.0;
    let duration = 2.0;

    let mut osc = super_square_oscillator::SuperSquareOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f0 = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let shape = modulation;
        osc.render(f0, shape, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/super_square.wav", &wav_data).ok();
}

#[test]
fn variable_saw_oscillator() {
    let frequency = 110.0;
    let waveshape = 1.0;
    let duration = 2.0;

    let mut osc = variable_saw_oscillator::VariableSawOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
        osc.render(f, pw, waveshape, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/variable_saw.wav", &wav_data).ok();
}

#[test]
fn variable_shape_oscillator() {
    let master_frequency = 110.0;
    let frequency = 110.0;
    let waveshape = 1.0;
    let duration = 2.0;

    let mut osc = variable_shape_oscillator::VariableShapeOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let master_f = master_frequency / SAMPLE_RATE;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let pw = modulation;
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
    let duration = 2.0;

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
    let frequency = 110.0;
    let duration = 10.0;

    let mut wavetable = [&mi_plaits_dsp::dsp::resources::WAV_INTEGRATED_WAVES[0..260]; 256];

    for (n, wt) in mi_plaits_dsp::dsp::resources::WAV_INTEGRATED_WAVES
        .chunks(260)
        .enumerate()
    {
        wavetable[n] = wt;
    }

    let mut osc = wavetable_oscillator::WavetableOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let waveform = modulation;
        out.fill(0.0);
        osc.render(f, 1.0, waveform, &wavetable, &mut out, 256, 192, true);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("oscillator/wavetable.wav", &wav_data).ok();
}

#[test]
fn z_oscillator() {
    let carrier_frequency = 80.0;
    let formant_frequency = 250.7;
    let carrier_shape = 0.5;
    let duration = 2.0;

    let mut osc = z_oscillator::ZOscillator::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    osc.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let carrier_f = carrier_frequency / SAMPLE_RATE;
    let formant_f = formant_frequency / SAMPLE_RATE;

    for n in 0..blocks {
        let modulation = modulation::ramp_up(n, blocks);
        let modulation_2 = modulation::ramp_up(n, blocks);
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
