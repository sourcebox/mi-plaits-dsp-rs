//! Phonemes table
//!
//! This is a table with vowels (a, e, i, o, u) and a random selection of
//! consonnants for the LPC10 speech synth.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::lpc_speech_synth::LpcSpeechSynthFrame;
use super::lpc_speech_synth_controller::NUM_PHONEMES;

pub const PHONEMES: [LpcSpeechSynthFrame; NUM_PHONEMES] = [
    LpcSpeechSynthFrame::new(192, 80, -18368, 11584, 52, 29, 23, 14, -17, 79, 37, 4),
    LpcSpeechSynthFrame::new(192, 80, -14528, 1536, 38, 29, 11, 14, -41, 79, 57, 4),
    LpcSpeechSynthFrame::new(192, 80, 14528, 9216, 25, -54, -70, 36, 19, 79, 57, 22),
    LpcSpeechSynthFrame::new(192, 80, -14528, -13440, 38, 57, 57, 14, -53, 7, 37, 77),
    LpcSpeechSynthFrame::new(192, 80, -26368, 4160, 11, 15, -1, 36, -41, 31, 77, 22),
    LpcSpeechSynthFrame::new(15, 0, 5184, 9216, -29, -12, 0, 0, 0, 0, 0, 0),
    LpcSpeechSynthFrame::new(10, 0, 27968, 17856, 25, 43, -24, -20, -53, 55, -4, -51),
    LpcSpeechSynthFrame::new(128, 160, 14528, -3712, -43, -26, -24, -20, -53, 55, -4, -51),
    LpcSpeechSynthFrame::new(128, 160, 10048, 11584, -16, 15, 0, 0, 0, 0, 0, 0),
    LpcSpeechSynthFrame::new(224, 100, 18368, -13440, -97, -26, -12, -53, -41, 7, 57, 32),
    LpcSpeechSynthFrame::new(192, 80, -10048, 9216, -70, 15, 34, -20, -17, 31, -24, 22),
    LpcSpeechSynthFrame::new(96, 160, -18368, 17856, -29, -12, -35, 3, -5, 7, 37, 22),
    LpcSpeechSynthFrame::new(64, 80, -21632, -6272, -83, 29, 57, 3, -5, 7, 16, 32),
    LpcSpeechSynthFrame::new(192, 80, 0, -1088, 11, -26, -24, -9, -5, 55, 37, 22),
    LpcSpeechSynthFrame::new(64, 80, 21632, -17536, -97, 85, 57, -20, -17, 31, -4, 59),
];
