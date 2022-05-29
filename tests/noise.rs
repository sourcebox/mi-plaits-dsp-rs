//! Tests for the noise generators

mod wav_writer;

use mi_plaits_dsp::dsp::noise::*;
use mi_plaits_dsp::dsp::SAMPLE_RATE;

const BLOCK_SIZE: usize = 24;

#[test]
fn clocked_noise() {
    let frequency = 10.0;
    let duration = 1.0;

    let mut noise = clocked_noise::ClockedNoise::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    noise.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for _ in 0..blocks {
        noise.render(false, f, &mut out);
        wav_data.extend_from_slice(&out);
    }

    wav_writer::write("noise/clocked.wav", &wav_data).ok();
}

#[test]
fn dust() {
    let frequency = 20.0;
    let duration = 1.0;

    let mut wav_data = Vec::new();

    let samples = (duration * SAMPLE_RATE) as usize;
    let f = frequency / SAMPLE_RATE;

    for _ in 0..samples {
        let out = dust::dust(f);
        wav_data.push(out);
    }

    wav_writer::write("noise/dust.wav", &wav_data).ok();
}

#[test]
fn particle() {
    let frequency = 50.0;
    let density = 0.1;
    let gain = 1.0;
    let spread = 0.5;
    let q = 0.9;
    let duration = 1.0;

    let mut noise = particle::Particle::new();
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut wav_data = Vec::new();
    let mut wav_data_aux = Vec::new();
    noise.init();

    let blocks = (duration * SAMPLE_RATE / (BLOCK_SIZE as f32)) as usize;
    let f = frequency / SAMPLE_RATE;

    for n in 0..blocks {
        out.fill(0.0);
        aux.fill(0.0);
        let sync = n % (blocks / 5) == 0;
        noise.render(sync, density, gain, f, spread, q, &mut out, &mut aux);
        wav_data.extend_from_slice(&out);
        wav_data_aux.extend_from_slice(&aux);
    }

    wav_writer::write("noise/particle.wav", &wav_data).ok();
    wav_writer::write("noise/particle_aux.wav", &wav_data_aux).ok();
}

#[test]
fn smooth_random_generator() {
    let frequency = 10.0;
    let duration = 1.0;

    let mut osc = smooth_random_generator::SmoothRandomGenerator::new();
    let mut wav_data = Vec::new();
    osc.init();

    let samples = (duration * SAMPLE_RATE) as usize;
    let f = frequency / SAMPLE_RATE;

    for _ in 0..samples {
        let out = osc.render(f);
        wav_data.push(out);
    }

    wav_writer::write("noise/smooth_random_generator.wav", &wav_data).ok();
}
