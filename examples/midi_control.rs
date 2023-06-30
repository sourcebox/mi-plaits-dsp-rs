//! Single voice with MIDI parameter control.

use audio_midi_shell::{AudioGenerator, AudioMidiShell};
use simple_logger::SimpleLogger;

use mi_plaits_dsp::dsp::voice::{Modulations, Patch, Voice};

const SAMPLE_RATE: u32 = 48000;
const BLOCK_SIZE: usize = 32;

fn main() -> ! {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    AudioMidiShell::run_forever(SAMPLE_RATE, BLOCK_SIZE, App::new());
}

#[derive(Debug)]
struct App<'a> {
    voice: Voice<'a>,
    patch: Patch,
    modulations: Modulations,
    volume: f32,
    balance: f32,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        Self {
            voice: Voice::new(&std::alloc::System, BLOCK_SIZE),
            patch: Patch::default(),
            modulations: Modulations::default(),
            volume: 1.0,
            balance: 0.0,
        }
    }
}

impl<'a> AudioGenerator for App<'a> {
    fn init(&mut self, _block_size: usize) {
        self.patch.engine = 0;
        self.patch.harmonics = 0.5;
        self.patch.timbre = 0.5;
        self.patch.morph = 0.5;
        self.modulations.trigger_patched = true;
        self.modulations.level_patched = true;
        self.voice.init();
    }

    fn process(&mut self, samples_left: &mut [f32], samples_right: &mut [f32]) {
        let mut out = vec![0.0; BLOCK_SIZE];
        let mut aux = vec![0.0; BLOCK_SIZE];

        self.voice
            .render(&self.patch, &self.modulations, &mut out, &mut aux);

        let mut mix = vec![0.0; BLOCK_SIZE];

        for frame in 0..BLOCK_SIZE {
            mix[frame] =
                (out[frame] * (1.0 - self.balance) + aux[frame] * self.balance) * self.volume;
        }

        samples_left.clone_from_slice(&mix);
        samples_right.clone_from_slice(&mix);
    }

    fn process_midi(&mut self, message: Vec<u8>) {
        match message[0] & 0xF0 {
            0x80 => {
                // Note off
                self.modulations.trigger = 0.0;
                self.modulations.level = 0.0;
                log::info!("Note off: {}", message[1]);
            }
            0x90 if message[2] != 0 => {
                // Note on
                self.patch.note = message[1] as f32;
                self.modulations.trigger = 1.0;
                self.modulations.level = message[2] as f32 / 127.0;
                log::info!("Note on: {}", message[1]);
            }
            0xB0 => {
                // Control change
                let value = message[2] as f32 / 127.0;
                match message[1] {
                    21 => {
                        self.patch.engine = (value * 23.0) as usize;
                        log::info!("Engine: {}", self.patch.engine);
                    }
                    22 => {
                        self.patch.harmonics = value;
                        log::info!("Harmonics: {}", self.patch.harmonics);
                    }
                    23 => {
                        self.patch.timbre = value;
                        log::info!("Timbre: {}", self.patch.timbre);
                    }
                    24 => {
                        self.patch.morph = value;
                        log::info!("Morph: {}", self.patch.morph);
                    }
                    25 => {
                        self.balance = value;
                        log::info!("Blend OUT/AUX: {}", self.balance);
                    }
                    26 => {
                        self.patch.decay = value;
                        log::info!("Env Decay: {}", self.patch.decay);
                    }
                    27 => {
                        self.patch.lpg_colour = value;
                        log::info!("LPG Color: {}", self.patch.lpg_colour);
                    }
                    28 => {
                        self.volume = value;
                        log::info!("Volume: {}", self.volume);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
