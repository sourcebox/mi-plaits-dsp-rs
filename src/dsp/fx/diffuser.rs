//! Granular diffuser.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

#[derive(Debug)]
pub struct Diffuser {
    // Todo: implement
}

impl Diffuser {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T) -> Self {
        Self {
        // Todo: implement
        }
    }

    pub fn init(&mut self) {
        // Todo: implement
    }

    pub fn clear(&mut self) {
        // Todo: implement
    }

    #[inline]
    pub fn process(&mut self, amount: f32, rt: f32, in_out: &mut [f32]) {
        // Todo: implement
    }
}
