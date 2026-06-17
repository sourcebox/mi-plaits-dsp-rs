//! Tests for the noise generators

mod common;

use common::*;
use mi_plaits_dsp::noise::*;

const BLOCK_SIZE: usize = 24;

#[test]
fn clocked_noise() {
    let frequency = 10.0;
    let duration = 1.0;

    for sample_rate in SAMPLE_RATES {
        let mut noise = clocked_noise::ClockedNoise::new();
        let mut out = [0.0; BLOCK_SIZE];
        let mut wav_data = Vec::new();
        noise.init();

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;
        let f = frequency / sample_rate as f32;

        for _ in 0..blocks {
            noise.render(false, f, &mut out);
            wav_data.extend_from_slice(&out);
        }

        let filename = format!("noises/clocked/clocked_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();
    }
}

#[test]
fn dust() {
    let frequency = 20.0;
    let duration = 1.0;

    for sample_rate in SAMPLE_RATES {
        let mut wav_data = Vec::new();

        let samples = (duration * sample_rate as f32) as usize;
        let f = frequency / sample_rate as f32;

        for _ in 0..samples {
            let out = dust::dust(f);
            wav_data.push(out);
        }

        let filename = format!("noises/dust/dust_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();
    }
}

#[test]
fn particle() {
    let frequency = 50.0;
    let density = 0.1;
    let gain = 1.0;
    let spread = 0.5;
    let q = 0.9;
    let duration = 1.0;

    for sample_rate in SAMPLE_RATES {
        let mut noise = particle::Particle::new();
        let mut out = [0.0; BLOCK_SIZE];
        let mut aux = [0.0; BLOCK_SIZE];
        let mut wav_data = Vec::new();
        let mut wav_data_aux = Vec::new();
        noise.init();

        let blocks = (duration * sample_rate as f32 / (BLOCK_SIZE as f32)) as usize;
        let f = frequency / sample_rate as f32;

        for n in 0..blocks {
            out.fill(0.0);
            aux.fill(0.0);
            let sync = n % (blocks / 5) == 0;
            noise.render(sync, density, gain, f, spread, q, &mut out, &mut aux);
            wav_data.extend_from_slice(&out);
            wav_data_aux.extend_from_slice(&aux);
        }

        let filename = format!("noises/particle/particle_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();

        let filename_aux = format!("noises/particle/particle_aux_{sample_rate}.wav");
        write_wav(filename_aux, &wav_data_aux, sample_rate).ok();
    }
}

#[test]
fn smooth_random_generator() {
    let frequency = 10.0;
    let duration = 1.0;

    for sample_rate in SAMPLE_RATES {
        let mut osc = smooth_random_generator::SmoothRandomGenerator::new();
        let mut wav_data = Vec::new();
        osc.init();

        let samples = (duration * sample_rate as f32) as usize;
        let f = frequency / sample_rate as f32;

        for _ in 0..samples {
            let out = osc.render(f);
            wav_data.push(out);
        }

        let filename =
            format!("noises/smooth_random_generator/smooth_random_generator_{sample_rate}.wav");
        write_wav(filename, &wav_data, sample_rate).ok();
    }
}
