//! FM Algorithms and how to render them.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::operator::RenderFn;

// Store information about all FM algorithms, and which functions to call
// to render them.
//
// The raw structure of each algorithm is stored as a sequence of "opcodes",
// where each opcode indicates, for each operator, from where does it get
// its phase modulation signal and to which buffer it writes the result.
// This data is compact - 1 byte / algorithm / operator.
//
// At run time, this data is "compiled" into a sequence of function calls
// to pre-compiled renderers. A renderer is specialized to efficiently render
// (without any branching, and as much loop unrolling as possible) one
// operator or a group of operators.
//
// Different code space and speed trade-off can be obtained by increasing the
// palette of available renderers (for example by specializing the code for
// a renderer rendering in a single pass a "tower" of 4 operators).

#[derive(Debug)]
pub struct Algorithms<const NUM_OPERATORS: usize, const NUM_ALGORITHMS: usize> {
    render_call: [[RenderCall; NUM_OPERATORS]; NUM_ALGORITHMS],
}

impl<const NUM_OPERATORS: usize, const NUM_ALGORITHMS: usize>
    Algorithms<NUM_OPERATORS, NUM_ALGORITHMS>
{
    #[inline]
    pub fn init(&mut self) {
        for i in 0..NUM_ALGORITHMS {
            self.compile(i as u32);
        }
    }

    #[inline]
    pub fn render_call(&self, algorithm: u32, op: u32) -> &RenderCall {
        &self.render_call[algorithm as usize][op as usize]
    }

    #[inline]
    pub fn is_modulator(&self, algorithm: u32, op: u32) -> bool {
        (self.opcode(algorithm, op) & OPCODE_DESTINATION_MASK) != 0
    }

    #[inline]
    fn get_renderer(&self, n: u32, modulation_source: i32, additive: bool) -> Option<RenderFn> {
        for renderer in self.renderers() {
            if renderer.n == n
                && renderer.modulation_source == modulation_source
                && renderer.additive == additive
            {
                return Some(renderer.render_fn);
            }
        }

        None
    }

    #[inline]
    fn compile(&mut self, algorithm: u32) {
        let mut i = 0;

        while i < NUM_OPERATORS {
            let opcode = self.opcode(algorithm, i as u32);

            let mut n = 1;

            while i + n < NUM_OPERATORS {
                let from = self.opcode(algorithm, (i + n - 1) as u32);
                let to = (self.opcode(algorithm, (i + n) as u32) & OPCODE_SOURCE_MASK) >> 4;

                let has_additive = (from & OPCODE_ADDITIVE_FLAG) != 0;
                let broken = (from & OPCODE_DESTINATION_MASK) != to;

                if has_additive || broken {
                    if to == (opcode & OPCODE_DESTINATION_MASK) {
                        // If the same modulation happens to be reused by subsequent
                        // operators (algorithms 19 to 25), discard the chain.
                        n = 1;
                    }
                    break;
                }

                n += 1;
            }

            // Try to find if a pre-compiled renderer is available for this chain.
            for _attempt in 0..2 {
                let out_opcode = self.opcode(algorithm, (i + n - 1) as u32);
                let additive = (out_opcode & OPCODE_ADDITIVE_FLAG) != 0;

                let mut modulation_source = -3;

                if (opcode & OPCODE_SOURCE_MASK) == 0 {
                    modulation_source = -1;
                } else if (opcode & OPCODE_SOURCE_MASK) != OPCODE_SOURCE_FEEDBACK {
                    modulation_source = -2;
                } else {
                    for j in 0..n {
                        if (self.opcode(algorithm, (i + j) as u32) & OPCODE_FEEDBACK_SOURCE_FLAG)
                            != 0
                        {
                            modulation_source = j as i32;
                        }
                    }
                }

                let fn_ = self.get_renderer(n as u32, modulation_source, additive);

                if let Some(fn_) = fn_ {
                    let mut call = &mut self.render_call[algorithm as usize][i];
                    call.render_fn = fn_;
                    call.n = n as u32;
                    call.input_index = ((opcode & OPCODE_SOURCE_MASK) >> 4) as u32;
                    call.output_index = (out_opcode & OPCODE_DESTINATION_MASK) as u32;
                    break;
                } else if n == 1 {
                    // assert(false);
                } else {
                    n = 1;
                }
            }

            i += n;
        }
    }

    fn opcode(&self, algorithm: u32, op: u32) -> u8 {
        todo!()
    }

    fn renderers(&self) -> &[RendererSpecs] {
        todo!()
    }
}

const OPCODE_DESTINATION_MASK: u8 = 0x03;
const OPCODE_SOURCE_MASK: u8 = 0x30;
const OPCODE_SOURCE_FEEDBACK: u8 = 0x30;
const OPCODE_ADDITIVE_FLAG: u8 = 0x04;
const OPCODE_FEEDBACK_SOURCE_FLAG: u8 = 0x40;

#[derive(Debug)]
pub struct RenderCall {
    render_fn: RenderFn,
    n: u32,
    input_index: u32,
    output_index: u32,
}

#[derive(Debug)]
struct RendererSpecs {
    n: u32,
    modulation_source: i32,
    additive: bool,
    render_fn: RenderFn,
}
