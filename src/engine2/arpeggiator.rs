//! Arpeggiator.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use crate::utils::random;

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum ArpeggiatorMode {
    #[default]
    Up,

    Down,
    UpDown,
    Random,
    Last,
}

impl<T> From<T> for ArpeggiatorMode
where
    T: Into<i32>,
{
    fn from(value: T) -> Self {
        match value.into() {
            1 => ArpeggiatorMode::Down,
            2 => ArpeggiatorMode::UpDown,
            3 => ArpeggiatorMode::Random,
            4 => ArpeggiatorMode::Last,
            _ => ArpeggiatorMode::Up,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Arpeggiator {
    mode: ArpeggiatorMode,
    range: i32,

    note: i32,
    octave: i32,
    direction: i32,
}

impl Arpeggiator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.mode = ArpeggiatorMode::Up;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.note = 0;
        self.octave = 0;
        self.direction = 1;
    }

    pub fn set_mode(&mut self, mode: ArpeggiatorMode) {
        self.mode = mode;
    }

    pub fn set_range(&mut self, range: i32) {
        self.range = range;
    }

    pub fn note(&self) -> i32 {
        self.note
    }

    pub fn octave(&self) -> i32 {
        self.octave
    }

    pub fn clock(&mut self, num_notes: i32) {
        if num_notes == 0 {
            return;
        }

        if num_notes == 1 && self.range == 1 {
            self.note = 0;
            self.octave = 0;
            return;
        }

        if self.mode == ArpeggiatorMode::Random {
            loop {
                let w = random::get_word();
                let octave = ((w >> 4) as i32) % self.range;
                let note = ((w >> 20) as i32) % num_notes;
                if octave != self.octave || note != self.note {
                    self.octave = octave;
                    self.note = note;
                    break;
                }
            }
            return;
        }

        if self.mode == ArpeggiatorMode::Up {
            self.direction = 1;
        }

        if self.mode == ArpeggiatorMode::Down {
            self.direction = -1;
        }

        self.note += self.direction;

        let mut done = false;

        while !done {
            done = true;

            if self.note >= num_notes || self.note < 0 {
                self.octave += self.direction;
                self.note = if self.direction > 0 { 0 } else { num_notes - 1 };
            }

            if self.octave >= self.range || self.octave < 0 {
                self.octave = if self.direction > 0 {
                    0
                } else {
                    self.range - 1
                };
                if self.mode == ArpeggiatorMode::UpDown {
                    self.direction = -self.direction;
                    self.note = if self.direction > 0 { 1 } else { num_notes - 2 };
                    self.octave = if self.direction > 0 {
                        0
                    } else {
                        self.range - 1
                    };
                    done = false;
                }
            }
        }

        self.note = self.note.max(0);
    }
}
