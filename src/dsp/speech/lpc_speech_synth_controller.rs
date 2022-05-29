//! Feeds frames to the LPC10 speech synth.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::{GlobalAlloc, Layout};

use super::lpc_speech_synth::{LpcSpeechSynth, LpcSpeechSynthFrame, LPC_SPEECH_SYNTH_DEFAULT_F0};
use super::lpc_speech_synth_phonemes::PHONEMES;
use super::lpc_speech_synth_words::{NUM_WORD_BANKS, WORD_BANKS};
use crate::dsp::{CORRECTED_SAMPLE_RATE, SAMPLE_RATE};
use crate::stmlib::dsp::parameter_interpolator::ParameterInterpolator;
use crate::stmlib::dsp::polyblep::{next_blep_sample, this_blep_sample};
use crate::stmlib::dsp::units::semitones_to_ratio;

const MAX_WORDS: usize = 32;
const MAX_FRAMES: usize = 1024;
const NUM_VOWELS: usize = 5;
const NUM_CONSONANTS: usize = 10;
pub const NUM_PHONEMES: usize = NUM_VOWELS + NUM_CONSONANTS;
const SYNTH_FPS: f32 = 40.0;

#[derive(Debug)]
pub struct LpcSpeechSynthController<'a> {
    clock_phase: f32,
    sample: [f32; 2],
    next_sample: [f32; 2],
    gain: f32,
    synth: LpcSpeechSynth,

    playback_frame: i32,
    last_playback_frame: i32,
    remaining_frame_samples: usize,

    word_bank: LpcSpeechSynthWordBank<'a>,
}

impl<'a> LpcSpeechSynthController<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T) -> Self {
        Self {
            clock_phase: 0.0,
            sample: [0.0; 2],
            next_sample: [0.0; 2],
            gain: 0.0,
            synth: LpcSpeechSynth::new(),
            playback_frame: -1,
            last_playback_frame: -1,
            remaining_frame_samples: 0,
            word_bank: LpcSpeechSynthWordBank::new(
                buffer_allocator,
                WORD_BANKS.as_slice(),
                NUM_WORD_BANKS,
            ),
        }
    }

    pub fn init(&mut self) {
        self.synth.init();
    }

    pub fn reset(&mut self) {
        self.word_bank.reset();
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        free_running: bool,
        trigger: bool,
        bank: i32,
        frequency: f32,
        prosody_amount: f32,
        speed: f32,
        address: f32,
        formant_shift: f32,
        gain: f32,
        excitation: &mut [f32],
        output: &mut [f32],
    ) {
        let rate_ratio = semitones_to_ratio((formant_shift - 0.5) * 36.0);
        let rate = rate_ratio / 6.0;

        // All utterances have been normalized for an average f0 of 100 Hz.
        let pitch_shift =
            frequency / (rate_ratio * LPC_SPEECH_SYNTH_DEFAULT_F0 / CORRECTED_SAMPLE_RATE);
        let time_stretch = semitones_to_ratio(
            -speed * 24.0
                + (if formant_shift < 0.4 {
                    (formant_shift - 0.4) * -45.0
                } else if formant_shift > 0.6 {
                    (formant_shift - 0.6) * -45.0
                } else {
                    0.0
                }),
        );

        if bank != -1 {
            let reset_everything = self.word_bank.load(bank as usize);
            if reset_everything {
                self.playback_frame = -1;
                self.last_playback_frame = -1;
            }
        }

        let num_frames = if bank == -1 {
            NUM_VOWELS
        } else {
            self.word_bank.num_frames()
        };

        if trigger {
            if bank == -1 {
                // Pick a pseudo-random consonant, and play it for the duration of a
                // frame.
                let r = (address + 3.0 * formant_shift + 7.0 * frequency) * 8.0;
                self.playback_frame = (r as usize % NUM_CONSONANTS) as i32;
                self.playback_frame += NUM_VOWELS as i32;
                self.last_playback_frame = self.playback_frame + 1;
            } else {
                self.word_bank.get_word_boundaries(
                    address,
                    &mut self.playback_frame,
                    &mut self.last_playback_frame,
                );
            }
            self.remaining_frame_samples = 0;
        }

        if self.playback_frame == -1 && self.remaining_frame_samples == 0 {
            let frames = if bank == -1 {
                PHONEMES.as_slice()
            } else {
                self.word_bank.frames()
            };
            self.synth
                .play_frame(frames, address * ((num_frames as f32) - 1.0001), true);
        } else {
            if self.remaining_frame_samples == 0 {
                let frames = if bank == -1 {
                    PHONEMES.as_slice()
                } else {
                    self.word_bank.frames()
                };
                self.synth
                    .play_frame(frames, self.playback_frame as f32, false);
                self.remaining_frame_samples = (SAMPLE_RATE / SYNTH_FPS * time_stretch) as usize;
                self.playback_frame += 1;
                if self.playback_frame >= self.last_playback_frame {
                    let back_to_scan_mode = bank == -1 || free_running;
                    self.playback_frame = if back_to_scan_mode {
                        -1
                    } else {
                        self.last_playback_frame
                    };
                }
            }
            self.remaining_frame_samples -= usize::min(output.len(), self.remaining_frame_samples);
        }

        let mut gain_modulation = ParameterInterpolator::new(&mut self.gain, gain, output.len());

        for (output_sample, excitation_sample) in output.iter_mut().zip(excitation.iter_mut()) {
            let mut this_sample = self.next_sample;
            self.next_sample.fill(0.0);

            self.clock_phase += rate;

            if self.clock_phase >= 1.0 {
                self.clock_phase -= 1.0;
                let reset_time = self.clock_phase / rate;
                let mut new_sample = [0.0; 2];
                let (new_sample_0, new_sample_1) = new_sample.split_at_mut(1);

                self.synth
                    .render(prosody_amount, pitch_shift, new_sample_0, new_sample_1);

                let discontinuity = [
                    new_sample[0] - self.sample[0],
                    new_sample[1] - self.sample[1],
                ];
                this_sample[0] += discontinuity[0] * this_blep_sample(reset_time);
                self.next_sample[0] += discontinuity[0] * next_blep_sample(reset_time);
                this_sample[1] += discontinuity[1] * this_blep_sample(reset_time);
                self.next_sample[1] += discontinuity[1] * next_blep_sample(reset_time);
                self.sample = new_sample;
            }

            self.next_sample[0] += self.sample[0];
            self.next_sample[1] += self.sample[1];
            let gain = gain_modulation.next();
            *excitation_sample = this_sample[0] * gain;
            *output_sample = this_sample[1] * gain;
        }
    }
}

#[derive(Debug)]
pub struct LpcSpeechSynthWordBankData<'a> {
    pub data: &'a [u8],
    pub size: usize,
}

#[derive(Debug)]
pub struct LpcSpeechSynthWordBank<'a> {
    word_banks: &'a [LpcSpeechSynthWordBankData<'a>],

    num_banks: usize,
    loaded_bank: Option<usize>,
    num_frames: usize,
    num_words: usize,
    word_boundaries: [usize; MAX_WORDS],

    frames: &'a mut [LpcSpeechSynthFrame],
}

impl<'a> LpcSpeechSynthWordBank<'a> {
    pub fn new<T: GlobalAlloc>(
        buffer_allocator: &T,
        word_banks: &'a [LpcSpeechSynthWordBankData<'a>],
        num_banks: usize,
    ) -> Self {
        Self {
            word_banks,
            num_banks,
            loaded_bank: None,
            num_frames: 0,
            num_words: 0,
            word_boundaries: [0; MAX_WORDS],
            frames: allocate_buffer(buffer_allocator, MAX_FRAMES),
        }
    }

    pub fn init(&mut self, word_banks: &'a [LpcSpeechSynthWordBankData<'a>], num_banks: usize) {
        self.word_banks = word_banks;
        self.num_banks = num_banks;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.loaded_bank = None;
        self.num_frames = 0;
        self.num_words = 0;
    }

    pub fn load(&mut self, bank: usize) -> bool {
        if bank >= self.num_banks {
            return false;
        }

        if let Some(loaded_bank) = self.loaded_bank {
            if bank == loaded_bank {
                return false;
            }
        }

        self.num_frames = 0;
        self.num_words = 0;

        let data = self.word_banks[bank].data;
        let mut size = self.word_banks[bank].size;

        let mut data_index = 0;

        while size > 0 {
            self.word_boundaries[self.num_words] = self.num_frames;
            let consumed = self.load_next_word(&data[data_index..]);

            data_index += consumed;
            size -= consumed;
            self.num_words += 1;
        }

        self.word_boundaries[self.num_words] = self.num_frames;
        self.loaded_bank = Some(bank);

        true
    }

    #[inline]
    fn load_next_word(&mut self, data: &[u8]) -> usize {
        let mut bitstream = BitStream::new(data);

        loop {
            let mut frame = LpcSpeechSynthFrame::default();
            let energy = bitstream.get_bits(4);

            if energy == 0 {
                frame.energy = 0;
            } else if energy == 0xf {
                bitstream.flush();
                break;
            } else {
                frame.energy = ENERGY_LUT[energy];
                let repeat = bitstream.get_bits(1);
                frame.period = PERIOD_LUT[bitstream.get_bits(6)];
                if repeat == 0 {
                    frame.k0 = K0_LUT[bitstream.get_bits(5)];
                    frame.k1 = K1_LUT[bitstream.get_bits(5)];
                    frame.k2 = K2_LUT[bitstream.get_bits(4)];
                    frame.k3 = K3_LUT[bitstream.get_bits(4)];
                    if frame.period != 0 {
                        frame.k4 = K4_LUT[bitstream.get_bits(4)];
                        frame.k5 = K5_LUT[bitstream.get_bits(4)];
                        frame.k6 = K6_LUT[bitstream.get_bits(4)];
                        frame.k7 = K7_LUT[bitstream.get_bits(3)];
                        frame.k8 = K8_LUT[bitstream.get_bits(3)];
                        frame.k9 = K9_LUT[bitstream.get_bits(3)];
                    }
                }
            }

            self.frames[self.num_frames] = frame;
            self.num_frames += 1;
        }

        bitstream.count()
    }

    #[inline]
    pub fn num_frames(&self) -> usize {
        self.num_frames
    }

    #[inline]
    pub fn frames(&mut self) -> &mut [LpcSpeechSynthFrame] {
        self.frames
    }

    #[inline]
    pub fn get_word_boundaries(&self, address: f32, start: &mut i32, end: &mut i32) {
        if self.num_words == 0 {
            *start = -1;
            *end = -1;
        } else {
            let mut word = (address * (self.num_words as f32)) as i32;
            if word >= self.num_words as i32 {
                word = self.num_words as i32 - 1;
            }
            *start = self.word_boundaries[word as usize] as i32;
            *end = self.word_boundaries[word as usize + 1] as i32 - 1;
        }
    }
}

struct BitStream<'a> {
    p: &'a [u8],
    available: i32,
    bits: u16,
    count: usize,
}

impl<'a> BitStream<'a> {
    pub fn new(p: &'a [u8]) -> Self {
        Self {
            p,
            available: 0,
            bits: 0,
            count: 0,
        }
    }

    #[inline]
    pub fn flush(&mut self) {
        while self.available > 0 {
            self.get_bits(1);
        }
    }

    #[inline]
    pub fn get_bits(&mut self, num_bits: i32) -> usize {
        let mut shift = num_bits;

        if num_bits > self.available {
            self.bits <<= self.available;
            shift -= self.available;
            self.bits |= Self::reverse(self.p[0]) as u16;
            self.p = &self.p[1..];
            self.count += 1;
            self.available += 8;
        }

        self.bits <<= shift;
        let result = (self.bits >> 8) as u8;
        self.bits &= 0xff;
        self.available -= num_bits;

        result as usize
    }

    pub fn count(&self) -> usize {
        self.count
    }

    #[inline]
    fn reverse(mut b: u8) -> u8 {
        b = (b >> 4) | (b << 4);
        b = ((b & 0xcc) >> 2) | ((b & 0x33) << 2);
        b = ((b & 0xaa) >> 1) | ((b & 0x55) << 1);

        b
    }
}

const ENERGY_LUT: [u8; 16] = [
    0x00, 0x02, 0x03, 0x04, 0x05, 0x07, 0x0a, 0x0f, 0x14, 0x20, 0x29, 0x39, 0x51, 0x72, 0xa1, 0xff,
];

const PERIOD_LUT: [u8; 64] = [
    0, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38,
    39, 40, 41, 42, 43, 45, 47, 49, 51, 53, 54, 57, 59, 61, 63, 66, 69, 71, 73, 77, 79, 81, 85, 87,
    92, 95, 99, 102, 106, 110, 115, 119, 123, 128, 133, 138, 143, 149, 154, 160,
];

const K0_LUT: [i16; 32] = [
    32064, -31872, -31808, -31680, -31552, -31424, -31232, -30848, -30592, -30336, -30016, -29696,
    -29376, -28928, -28480, -27968, -26368, -24256, -21632, -18368, -14528, -10048, -5184, 0, 5184,
    10048, 14528, 18368, 21632, 24256, 26368, 27968,
];

const K1_LUT: [i16; 32] = [
    -20992, -19328, -17536, -15552, -13440, -11200, -8768, -6272, -3712, -1088, 1536, 4160, 6720,
    9216, 11584, 13824, 15936, 17856, 19648, 21248, 22656, 24000, 25152, 26176, 27072, 27840,
    28544, 29120, 29632, 30080, 30464, 32384,
];

const K2_LUT: [i8; 16] = [
    -110, -97, -83, -70, -56, -43, -29, -16, -2, 11, 25, 38, 52, 65, 79, 92,
];

const K3_LUT: [i8; 16] = [
    -82, -68, -54, -40, -26, -12, 1, 15, 29, 43, 57, 71, 85, 99, 113, 126,
];

const K4_LUT: [i8; 16] = [
    -82, -70, -59, -47, -35, -24, -12, -1, 11, 23, 34, 46, 57, 69, 81, 92,
];

const K5_LUT: [i8; 16] = [
    -64, -53, -42, -31, -20, -9, 3, 14, 25, 36, 47, 58, 69, 80, 91, 102,
];

const K6_LUT: [i8; 16] = [
    -77, -65, -53, -41, -29, -17, -5, 7, 19, 31, 43, 55, 67, 79, 90, 102,
];

const K7_LUT: [i8; 8] = [-64, -40, -16, 7, 31, 55, 79, 102];

const K8_LUT: [i8; 8] = [-64, -44, -24, -4, 16, 37, 57, 77];

const K9_LUT: [i8; 8] = [-51, -33, -15, 4, 22, 32, 59, 77];

pub fn allocate_buffer<T: GlobalAlloc>(
    buffer_allocator: &T,
    buffer_length: usize,
) -> &'static mut [LpcSpeechSynthFrame] {
    let size = buffer_length * core::mem::size_of::<LpcSpeechSynthFrame>();
    let buffer = unsafe {
        buffer_allocator.alloc_zeroed(Layout::from_size_align(size, 8).unwrap())
            as *mut LpcSpeechSynthFrame
    };
    let buffer: &mut [LpcSpeechSynthFrame] =
        unsafe { core::slice::from_raw_parts_mut(buffer, buffer_length) };

    buffer
}
