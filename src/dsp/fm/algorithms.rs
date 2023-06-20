//! FM Algorithms and how to render them.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::operator::{render_operators, RenderFn};

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

impl<const NUM_OPERATORS: usize, const NUM_ALGORITHMS: usize> Default
    for Algorithms<NUM_OPERATORS, NUM_ALGORITHMS>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const NUM_OPERATORS: usize, const NUM_ALGORITHMS: usize>
    Algorithms<NUM_OPERATORS, NUM_ALGORITHMS>
{
    pub fn new() -> Self {
        Self {
            render_call: core::array::from_fn(|_| core::array::from_fn(|_| RenderCall::new())),
        }
    }

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
                return renderer.render_fn;
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
                    call.render_fn = Some(fn_);
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
        if NUM_OPERATORS == 4 && NUM_ALGORITHMS == 8 {
            OPCODES_4_8[algorithm as usize][op as usize]
        } else if NUM_OPERATORS == 6 && NUM_ALGORITHMS == 32 {
            OPCODES_6_32[algorithm as usize][op as usize]
        } else {
            panic!("Unsupported configuration of operators and algorithms.");
        }
    }

    fn renderers(&self) -> &[RendererSpecs] {
        if NUM_OPERATORS == 4 && NUM_ALGORITHMS == 8 {
            &RENDERERS_4
        } else if NUM_OPERATORS == 6 && NUM_ALGORITHMS == 32 {
            &RENDERERS_6
        } else {
            panic!("Unsupported configuration of operators and algorithms.");
        }
    }
}

const OPCODE_DESTINATION_MASK: u8 = 0x03;
const OPCODE_SOURCE_MASK: u8 = 0x30;
const OPCODE_SOURCE_FEEDBACK: u8 = 0x30;
const OPCODE_ADDITIVE_FLAG: u8 = 0x04;
const OPCODE_FEEDBACK_SOURCE_FLAG: u8 = 0x40;

#[derive(Debug, Default)]
pub struct RenderCall {
    pub render_fn: Option<RenderFn>,
    pub n: u32,
    pub input_index: u32,
    pub output_index: u32,
}

impl RenderCall {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug)]
struct RendererSpecs {
    n: u32,
    modulation_source: i32,
    additive: bool,
    render_fn: Option<RenderFn>,
}

macro_rules! MOD {
    ($n:expr) => {
        $n << 4
    };
}

macro_rules! ADD {
    ($n:expr) => {
        $n | OPCODE_ADDITIVE_FLAG
    };
}

macro_rules! OUT {
    ($n:expr) => {
        $n
    };
}

macro_rules! FB_SRC {
    () => {
        OPCODE_FEEDBACK_SOURCE_FLAG
    };
}

macro_rules! FB_DST {
    () => {
        MOD!(3)
    };
}

macro_rules! FB {
    () => {
        FB_SRC!() | FB_DST!()
    };
}

macro_rules! NO_MOD {
    () => {
        MOD!(0)
    };
}

macro_rules! OUTPUT {
    () => {
        ADD!(0)
    };
}

#[allow(clippy::identity_op)]
const OPCODES_4_8: [[u8; 4]; 8] = [
    [
        // Algorithm 1: 4 -> 3 -> 2 -> 1
        FB!() | OUT!(1),
        MOD!(1) | OUT!(1),
        MOD!(1) | OUT!(1),
        MOD!(1) | OUTPUT!(),
    ],
    [
        // Algorithm 2: 4 + 3 -> 2 -> 1
        FB!() | OUT!(1),
        ADD!(1),
        MOD!(1) | OUT!(1),
        MOD!(1) | OUTPUT!(),
    ],
    [
        // Algorithm 3: 4 + (3 -> 2) -> 1
        FB!() | OUT!(1),
        OUT!(2),
        MOD!(2) | ADD!(1),
        MOD!(1) | OUTPUT!(),
    ],
    [
        // Algorithm 4: (4 -> 3) + 2 -> 1
        FB!() | OUT!(1),
        MOD!(1) | OUT!(1),
        ADD!(1),
        MOD!(1) | OUTPUT!(),
    ],
    [
        // Algorithm 5: (4 -> 3) + (2 -> 1)
        FB!() | OUT!(1),
        MOD!(1) | OUTPUT!(),
        OUT!(1),
        MOD!(1) | ADD!(0),
    ],
    [
        // Algorithm 6: (4 -> 3) + (4 -> 2) + (4 -> 1)
        FB!() | OUT!(1),
        MOD!(1) | OUTPUT!(),
        MOD!(1) | ADD!(0),
        MOD!(1) | ADD!(0),
    ],
    [
        // Algorithm 7: (4 -> 3) + 2 + 1
        FB!() | OUT!(1),
        MOD!(1) | OUTPUT!(),
        ADD!(0),
        ADD!(0),
    ],
    [
        // Algorithm 8: 4 + 3 + 2 + 1
        FB!() | OUTPUT!(),
        ADD!(0),
        ADD!(0),
        ADD!(0),
    ],
];

#[allow(clippy::identity_op)]
const OPCODES_6_32: [[u8; 6]; 32] = [
    [
        // Algorithm 1
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUT!(1),   // Op 5
        MOD!(1) | OUT!(1),   // Op 4
        MOD!(1) | OUTPUT!(), // Op 3
        NO_MOD!() | OUT!(1), // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 2
        NO_MOD!() | OUT!(1), // Op 6
        MOD!(1) | OUT!(1),   // Op 5
        MOD!(1) | OUT!(1),   // Op 4
        MOD!(1) | OUTPUT!(), // Op 3
        FB!() | OUT!(1),     // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 3
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUT!(1),   // Op 5
        MOD!(1) | OUTPUT!(), // Op 4
        NO_MOD!() | OUT!(1), // Op 3
        MOD!(1) | OUT!(1),   // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 4
        FB_DST!() | NO_MOD!() | OUT!(1), // Op 6
        MOD!(1) | OUT!(1),               // Op 5
        FB_SRC!() | MOD!(1) | OUTPUT!(), // Op 4
        NO_MOD!() | OUT!(1),             // Op 3
        MOD!(1) | OUT!(1),               // Op 2
        MOD!(1) | ADD!(0),               // Op 1
    ],
    [
        // Algorithm 5
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUTPUT!(), // Op 5
        NO_MOD!() | OUT!(1), // Op 4
        MOD!(1) | ADD!(0),   // Op 3
        NO_MOD!() | OUT!(1), // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 6
        FB_DST!() | NO_MOD!() | OUT!(1), // Op 6
        FB_SRC!() | MOD!(1) | OUTPUT!(), // Op 5
        NO_MOD!() | OUT!(1),             // Op 4
        MOD!(1) | ADD!(0),               // Op 3
        NO_MOD!() | OUT!(1),             // Op 2
        MOD!(1) | ADD!(0),               // Op 1
    ],
    [
        // Algorithm 7
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUT!(1),   // Op 5
        NO_MOD!() | ADD!(1), // Op 4
        MOD!(1) | OUTPUT!(), // Op 3
        NO_MOD!() | OUT!(1), // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 8
        NO_MOD!() | OUT!(1), // Op 6
        MOD!(1) | OUT!(1),   // Op 5
        FB!() | ADD!(1),     // Op 4
        MOD!(1) | OUTPUT!(), // Op 3
        NO_MOD!() | OUT!(1), // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 9
        NO_MOD!() | OUT!(1), // Op 6
        MOD!(1) | OUT!(1),   // Op 5
        NO_MOD!() | ADD!(1), // Op 4
        MOD!(1) | OUTPUT!(), // Op 3
        FB!() | OUT!(1),     // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 10
        NO_MOD!() | OUT!(1), // Op 6
        NO_MOD!() | ADD!(1), // Op 5
        MOD!(1) | OUTPUT!(), // Op 4
        FB!() | OUT!(1),     // Op 3
        MOD!(1) | OUT!(1),   // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 11
        FB!() | OUT!(1),     // Op 6
        NO_MOD!() | ADD!(1), // Op 5
        MOD!(1) | OUTPUT!(), // Op 4
        NO_MOD!() | OUT!(1), // Op 3
        MOD!(1) | OUT!(1),   // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 12
        NO_MOD!() | OUT!(1), // Op 6
        NO_MOD!() | ADD!(1), // Op 5
        NO_MOD!() | ADD!(1), // Op 4
        MOD!(1) | OUTPUT!(), // Op 3
        FB!() | OUT!(1),     // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 13
        FB!() | OUT!(1),     // Op 6
        NO_MOD!() | ADD!(1), // Op 5
        NO_MOD!() | ADD!(1), // Op 4
        MOD!(1) | OUTPUT!(), // Op 3
        NO_MOD!() | OUT!(1), // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 14
        FB!() | OUT!(1),     // Op 6
        NO_MOD!() | ADD!(1), // Op 5
        MOD!(1) | OUT!(1),   // Op 4
        MOD!(1) | OUTPUT!(), // Op 3
        NO_MOD!() | OUT!(1), // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 15
        NO_MOD!() | OUT!(1), // Op 6
        NO_MOD!() | ADD!(1), // Op 5
        MOD!(1) | OUT!(1),   // Op 4
        MOD!(1) | OUTPUT!(), // Op 3
        FB!() | OUT!(1),     // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 16
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUT!(1),   // Op 5
        NO_MOD!() | OUT!(2), // Op 4
        MOD!(2) | ADD!(1),   // Op 3
        NO_MOD!() | ADD!(1), // Op 2
        MOD!(1) | OUTPUT!(), // Op 1
    ],
    [
        // Algorithm 17
        NO_MOD!() | OUT!(1), // Op 6
        MOD!(1) | OUT!(1),   // Op 5
        NO_MOD!() | OUT!(2), // Op 4
        MOD!(2) | ADD!(1),   // Op 3
        FB!() | ADD!(1),     // Op 2
        MOD!(1) | OUTPUT!(), // Op 1
    ],
    [
        // Algorithm 18
        NO_MOD!() | OUT!(1), // Op 6
        MOD!(1) | OUT!(1),   // Op 5
        MOD!(1) | OUT!(1),   // Op 4
        FB!() | ADD!(1),     // Op 3
        NO_MOD!() | ADD!(1), // Op 2
        MOD!(1) | OUTPUT!(), // Op 1
    ],
    [
        // Algorithm 19
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUTPUT!(), // Op 5
        MOD!(1) | ADD!(0),   // Op 4
        NO_MOD!() | OUT!(1), // Op 3
        MOD!(1) | OUT!(1),   // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 20
        NO_MOD!() | OUT!(1), // Op 6
        NO_MOD!() | ADD!(1), // Op 5
        MOD!(1) | OUTPUT!(), // Op 4
        FB!() | OUT!(1),     // Op 3
        MOD!(1) | ADD!(0),   // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 21
        NO_MOD!() | OUT!(1), // Op 6
        MOD!(1) | OUTPUT!(), // Op 5
        MOD!(1) | ADD!(0),   // Op 4
        FB!() | OUT!(1),     // Op 3
        MOD!(1) | ADD!(0),   // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 22
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUTPUT!(), // Op 5
        MOD!(1) | ADD!(0),   // Op 4
        MOD!(1) | ADD!(0),   // Op 3
        NO_MOD!() | OUT!(1), // Op 2
        MOD!(1) | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 23
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUTPUT!(), // Op 5
        MOD!(1) | ADD!(0),   // Op 4
        NO_MOD!() | OUT!(1), // Op 3
        MOD!(1) | ADD!(0),   // Op 2
        NO_MOD!() | ADD!(0), // Op 1
    ],
    [
        // Algorithm 24
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUTPUT!(), // Op 5
        MOD!(1) | ADD!(0),   // Op 4
        MOD!(1) | ADD!(0),   // Op 3
        NO_MOD!() | ADD!(0), // Op 2
        NO_MOD!() | ADD!(0), // Op 1
    ],
    [
        // Algorithm 25
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUTPUT!(), // Op 5
        MOD!(1) | ADD!(0),   // Op 4
        NO_MOD!() | ADD!(0), // Op 3
        NO_MOD!() | ADD!(0), // Op 2
        NO_MOD!() | ADD!(0), // Op 1
    ],
    [
        // Algorithm 26
        FB!() | OUT!(1),     // Op 6
        NO_MOD!() | ADD!(1), // Op 5
        MOD!(1) | OUTPUT!(), // Op 4
        NO_MOD!() | OUT!(1), // Op 3
        MOD!(1) | ADD!(0),   // Op 2
        NO_MOD!() | ADD!(0), // Op 1
    ],
    [
        // Algorithm 27
        NO_MOD!() | OUT!(1), // Op 6
        NO_MOD!() | ADD!(1), // Op 5
        MOD!(1) | OUTPUT!(), // Op 4
        FB!() | OUT!(1),     // Op 3
        MOD!(1) | ADD!(0),   // Op 2
        NO_MOD!() | ADD!(0), // Op 1
    ],
    [
        // Algorithm 28
        NO_MOD!() | OUTPUT!(), // Op 6
        FB!() | OUT!(1),       // Op 5
        MOD!(1) | OUT!(1),     // Op 4
        MOD!(1) | ADD!(0),     // Op 3
        NO_MOD!() | OUT!(1),   // Op 2
        MOD!(1) | ADD!(0),     // Op 1
    ],
    [
        // Algorithm 29
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUTPUT!(), // Op 5
        NO_MOD!() | OUT!(1), // Op 4
        MOD!(1) | ADD!(0),   // Op 3
        NO_MOD!() | ADD!(0), // Op 2
        NO_MOD!() | ADD!(0), // Op 1
    ],
    [
        // Algorithm 30
        NO_MOD!() | OUTPUT!(), // Op 6
        FB!() | OUT!(1),       // Op 5
        MOD!(1) | OUT!(1),     // Op 4
        MOD!(1) | ADD!(0),     // Op 3
        NO_MOD!() | ADD!(0),   // Op 2
        NO_MOD!() | ADD!(0),   // Op 1
    ],
    [
        // Algorithm 31
        FB!() | OUT!(1),     // Op 6
        MOD!(1) | OUTPUT!(), // Op 5
        NO_MOD!() | ADD!(0), // Op 4
        NO_MOD!() | ADD!(0), // Op 3
        NO_MOD!() | ADD!(0), // Op 2
        NO_MOD!() | ADD!(0), // Op 1
    ],
    [
        // Algorithm 32
        FB!() | OUTPUT!(),   // Op 6
        NO_MOD!() | ADD!(0), // Op 5
        NO_MOD!() | ADD!(0), // Op 4
        NO_MOD!() | ADD!(0), // Op 3
        NO_MOD!() | ADD!(0), // Op 2
        NO_MOD!() | ADD!(0), // Op 1
    ],
];

macro_rules! INSTANTIATE_RENDERER {
    ($n:expr, $m:expr, $a: expr) => {
        RendererSpecs {
            n: $n,
            modulation_source: $m,
            additive: $a,
            render_fn: Some(render_operators::<$n, $m, $a>),
        }
    };
}

const RENDERERS_4: [RendererSpecs; 7] = [
    // Core
    INSTANTIATE_RENDERER!(1, -2, false),
    INSTANTIATE_RENDERER!(1, -2, true),
    INSTANTIATE_RENDERER!(1, -1, false),
    INSTANTIATE_RENDERER!(1, -1, true),
    INSTANTIATE_RENDERER!(1, 0, false),
    INSTANTIATE_RENDERER!(1, 0, true),
    RendererSpecs {
        n: 0,
        modulation_source: 0,
        additive: false,
        render_fn: None,
    },
];

const RENDERERS_6: [RendererSpecs; 9] = [
    // Core
    INSTANTIATE_RENDERER!(1, -2, false),
    INSTANTIATE_RENDERER!(1, -2, true),
    INSTANTIATE_RENDERER!(1, -1, false),
    INSTANTIATE_RENDERER!(1, -1, true),
    INSTANTIATE_RENDERER!(1, 0, false),
    INSTANTIATE_RENDERER!(1, 0, true),
    // Pesky feedback loops spanning several operators
    INSTANTIATE_RENDERER!(3, 2, true),
    INSTANTIATE_RENDERER!(2, 1, true),
    RendererSpecs {
        n: 0,
        modulation_source: 0,
        additive: false,
        render_fn: None,
    },
];
