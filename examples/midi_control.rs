//! Multiple voices with MIDI parameter control.

use audio_midi_shell::{AudioGenerator, AudioMidiShell};
use simple_logger::SimpleLogger;

use mi_plaits_dsp::voice::{Modulations, Patch, Voice};

const SAMPLE_RATE: u32 = 48000;
const BUFFER_SIZE: usize = 512;
const CHUNK_SIZE: usize = 24;
const VOICE_COUNT: usize = 8;

fn main() -> ! {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    AudioMidiShell::run_forever(SAMPLE_RATE, BUFFER_SIZE, CHUNK_SIZE, App::new());
}

#[derive(Debug)]
struct App<'a> {
    voices: Box<[Voice<'a>; VOICE_COUNT]>,
    patches: [Patch; VOICE_COUNT],
    modulations: [Modulations; VOICE_COUNT],
    volume: f32,
    balance: f32,
    note_map: [Option<u8>; VOICE_COUNT],
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        Self {
            voices: Box::new(core::array::from_fn(|_| Voice::new(CHUNK_SIZE))),
            patches: core::array::from_fn(|_| Patch::default()),
            modulations: core::array::from_fn(|_| Modulations::default()),
            volume: 1.0,
            balance: 0.0,
            note_map: [None; VOICE_COUNT],
        }
    }
}

impl<'a> AudioGenerator for App<'a> {
    fn init(&mut self, _block_size: usize) {
        for (n, voice) in self.voices.iter_mut().enumerate() {
            self.patches[n].engine = 0;
            self.patches[n].harmonics = 0.5;
            self.patches[n].timbre = 0.5;
            self.patches[n].morph = 0.5;
            self.modulations[n].trigger_patched = true;
            self.modulations[n].level_patched = true;
            voice.init();
        }
    }

    fn process(&mut self, frames: &mut [[f32; 2]]) {
        let mut mix = vec![0.0; CHUNK_SIZE];

        for (n, voice) in self.voices.iter_mut().enumerate() {
            let mut out = vec![0.0; CHUNK_SIZE];
            let mut aux = vec![0.0; CHUNK_SIZE];

            voice.render(&self.patches[n], &self.modulations[n], &mut out, &mut aux);

            for frame in 0..CHUNK_SIZE {
                mix[frame] +=
                    (out[frame] * (1.0 - self.balance) + aux[frame] * self.balance) * self.volume;
            }
        }

        for (n, frame) in frames.iter_mut().enumerate() {
            frame[0] = mix[n];
            frame[1] = mix[n];
        }
    }

    fn process_midi(&mut self, message: &[u8], _timestamp: u64) {
        match message[0] & 0xF0 {
            0x80 => {
                // Note off
                let note = message[1];
                while let Some(voice_no) = self
                    .note_map
                    .iter()
                    .position(|n| n.is_some() && n.unwrap() == note)
                {
                    self.modulations[voice_no].trigger = 0.0;
                    self.modulations[voice_no].level = 0.0;
                    self.note_map[voice_no] = None;
                    log::info!("Note off: {} on voice {}", note, voice_no);
                }
            }
            0x90 if message[2] != 0 => {
                // Note on
                let note = message[1];
                let voice_no = self
                    .note_map
                    .iter()
                    .position(|n| n.is_none())
                    .unwrap_or_default();
                self.patches[voice_no].note = note as f32;
                self.modulations[voice_no].trigger = 1.0;
                self.modulations[voice_no].level = message[2] as f32 / 127.0;
                self.note_map[voice_no] = Some(note);
                log::info!("Note on: {} on voice {}", note, voice_no);
            }
            0xB0 => {
                // Control change
                let value = message[2] as f32 / 127.0;
                match message[1] {
                    21 => {
                        let engine = (value * 23.0) as usize;
                        log::info!("Engine: {}", engine);
                        for patch in self.patches.iter_mut() {
                            patch.engine = engine;
                        }
                    }
                    22 => {
                        log::info!("Harmonics: {}", value);
                        for patch in self.patches.iter_mut() {
                            patch.harmonics = value;
                        }
                    }
                    23 => {
                        log::info!("Timbre: {}", value);
                        for patch in self.patches.iter_mut() {
                            patch.timbre = value;
                        }
                    }
                    24 => {
                        log::info!("Morph: {}", value);
                        for patch in self.patches.iter_mut() {
                            patch.morph = value;
                        }
                    }
                    25 => {
                        self.balance = value;
                        log::info!("Blend OUT/AUX: {}", self.balance);
                    }
                    26 => {
                        log::info!("Env Decay: {}", value);
                        for patch in self.patches.iter_mut() {
                            patch.decay = value;
                        }
                    }
                    27 => {
                        log::info!("LPG Color: {}", value);
                        for patch in self.patches.iter_mut() {
                            patch.lpg_colour = value;
                        }
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
