//! Granular diffuser.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use super::{DataFormat12Bit, FxContext, FxEngine};
use crate::utils::delay_line::DelayLine;
use crate::utils::{scaled_smoothing_coefficient, REFERENCE_SAMPLE_RATE};

#[derive(Debug, Default, Clone)]
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

    sample_rate_hz: f32,

    // Sample rate dependent constants
    delay_scale: f32,
    klp: f32,
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
            sample_rate_hz: 48000.0,
            delay_scale: 1.0,
            klp: 0.75,
        }
    }

    pub fn init(&mut self, sample_rate_hz: f32) {
        self.sample_rate_hz = sample_rate_hz;
        self.engine.set_lfo_frequency(0.3 / sample_rate_hz);
        self.lp_decay = 0.0;

        // Keep delay/tap times constant in seconds and the absorption filter
        // cutoff constant in Hz at any sample rate.
        self.delay_scale = sample_rate_hz / REFERENCE_SAMPLE_RATE;
        self.klp = scaled_smoothing_coefficient(0.75, REFERENCE_SAMPLE_RATE / sample_rate_hz);
        self.ap1.set_scale(self.delay_scale);
        self.ap2.set_scale(self.delay_scale);
        self.ap3.set_scale(self.delay_scale);
        self.ap4.set_scale(self.delay_scale);
        self.dapa.set_scale(self.delay_scale);
        self.dapb.set_scale(self.delay_scale);
        self.del.set_scale(self.delay_scale);
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
        let klp = self.klp;
        let scale = self.delay_scale;
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
            c.interpolate(&mut self.ap4, 400.0 * scale, 43.0 * scale, kap);
            c.write_all_pass(&mut self.ap4, -kap);
            c.interpolate(&mut self.del, 3070.0 * scale, 340.0 * scale, rt);
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
