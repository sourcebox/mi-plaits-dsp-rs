//! Granular diffuser.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{DataFormat12Bit, FxContext, FxEngine};
use crate::utils::delay_line::DelayLine;
use crate::SAMPLE_RATE;

#[derive(Debug, Default)]
pub struct Diffuser {
    ap1: DelayLine<i16, 126>,
    ap2: DelayLine<i16, 180>,
    ap3: DelayLine<i16, 269>,
    ap4: DelayLine<i16, 444>,
    dapa: DelayLine<i16, 1653>,
    dapb: DelayLine<i16, 2010>,
    del: DelayLine<i16, 3411>,

    engine: FxEngine<8192, DataFormat12Bit>,
    lp_decay: f32,
}

impl Diffuser {
    pub fn new() -> Self {
        Self {
            ap1: DelayLine::new(),
            ap2: DelayLine::new(),
            ap3: DelayLine::new(),
            ap4: DelayLine::new(),
            dapa: DelayLine::new(),
            dapb: DelayLine::new(),
            del: DelayLine::new(),

            engine: FxEngine::new(),
            lp_decay: 0.0,
        }
    }

    pub fn init(&mut self) {
        self.engine.set_lfo_frequency(0.3 / SAMPLE_RATE);
        self.lp_decay = 0.0;
    }

    pub fn reset(&mut self) {
        self.engine.clear();
    }

    pub fn clear(&mut self) {
        self.ap1.reset();
        self.ap2.reset();
        self.ap3.reset();
        self.ap4.reset();
        self.dapa.reset();
        self.dapb.reset();
        self.del.reset();
        self.engine.clear();
    }

    #[inline]
    pub fn process(&mut self, amount: f32, rt: f32, in_out: &mut [f32]) {
        let mut c = FxContext::new();

        let kap = 0.625;
        let klp = 0.75;
        let mut lp = self.lp_decay;

        for in_out_sample in in_out.iter_mut() {
            self.engine.start(&mut c);

            c.read(*in_out_sample);
            c.read_line(&mut self.ap1, kap);
            c.write_all_pass(&mut self.ap1, -kap);
            c.read_line(&mut self.ap2, kap);
            c.write_all_pass(&mut self.ap2, -kap);
            c.read_line(&mut self.ap3, kap);
            c.write_all_pass(&mut self.ap3, -kap);
            c.interpolate(&mut self.ap4, 400.0, 43.0, kap);
            c.write_all_pass(&mut self.ap4, -kap);
            c.interpolate(&mut self.del, 3070.0, 340.0, rt);
            c.lp(&mut lp, klp);
            c.read_line(&mut self.dapa, -kap);
            c.write_all_pass(&mut self.dapa, kap);
            c.read_line(&mut self.dapb, kap);
            c.write_all_pass(&mut self.dapb, -kap);
            c.write_line(&mut self.del, 2.0);

            let mut wet = 0.0;
            c.write_with_scale(&mut wet, 0.0);

            *in_out_sample += amount * (wet - *in_out_sample);
        }

        self.lp_decay = lp;
    }
}
