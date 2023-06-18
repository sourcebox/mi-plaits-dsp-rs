//! DX7 patch.

// Based on MIT-licensed code (c) 2021 by Emilie Gillet (emilie.o.gillet@gmail.com)

#[derive(Debug, Default)]
pub struct Patch {
    pub op: [Operator; 6],
    pub pitch_envelope: Envelope,
    pub algorithm: u8,
    pub feedback: u8,
    pub reset_phase: u8,
    pub modulations: ModulationParameters,
    pub transpose: u8,
    pub name: [u8; 10],
    pub active_operators: u8,
}

impl Patch {
    pub fn unpack(&mut self, data: &[u8]) {
        for (i, op) in self.op.iter_mut().enumerate() {
            let op_data = &data[(i * 17)..];

            for (j, (rate, level)) in op
                .envelope
                .rate
                .iter_mut()
                .zip(op.envelope.level.iter_mut())
                .enumerate()
            {
                *rate = u8::min(op_data[j] & 0x7F, 99);
                *level = u8::min(op_data[4 + j] & 0x7F, 99);
            }

            op.keyboard_scaling.break_point = u8::min(op_data[8] & 0x7F, 99);
            op.keyboard_scaling.left_depth = u8::min(op_data[9] & 0x7F, 99);
            op.keyboard_scaling.right_depth = u8::min(op_data[10] & 0x7F, 99);
            op.keyboard_scaling.left_curve = op_data[11] & 0x03;
            op.keyboard_scaling.right_curve = (op_data[11] >> 2) & 0x03;

            op.rate_scaling = op_data[12] & 0x07;
            op.amp_mod_sensitivity = op_data[13] & 0x03;
            op.velocity_sensitivity = (op_data[13] >> 2) & 0x07;
            op.level = u8::min(op_data[14] & 0x7F, 99);
            op.mode = op_data[15] & 0x01;
            op.coarse = (op_data[15] >> 1) & 0x1F;
            op.fine = u8::min(op_data[16] & 0x7F, 99);
            op.detune = u8::min((op_data[12] >> 3) & 0x0F, 14);
        }

        for (j, (rate, level)) in self
            .pitch_envelope
            .rate
            .iter_mut()
            .zip(self.pitch_envelope.level.iter_mut())
            .enumerate()
        {
            *rate = u8::min(data[102 + j] & 0x7F, 99);
            *level = u8::min(data[106 + j] & 0x7F, 99);
        }

        self.algorithm = data[110] & 0x1F;
        self.feedback = data[111] & 0x07;
        self.reset_phase = (data[111] >> 3) & 0x01;

        self.modulations.rate = u8::min(data[112] & 0x7F, 99);
        self.modulations.delay = u8::min(data[113] & 0x7F, 99);
        self.modulations.pitch_mod_depth = u8::min(data[114] & 0x7F, 99);
        self.modulations.amp_mod_depth = u8::min(data[115] & 0x7F, 99);
        self.modulations.reset_phase = data[116] & 0x01;
        self.modulations.waveform = u8::min((data[116] >> 1) & 0x07, 5);
        self.modulations.pitch_mod_sensitivity = data[116] >> 4;

        self.transpose = u8::min(data[117] & 0x7F, 48);

        for (i, c) in self.name.iter_mut().enumerate() {
            *c = data[118 + i] & 0x7F;
        }

        self.active_operators = 0x3F;
    }
}

#[derive(Debug, Default)]
pub struct Envelope {
    pub rate: [u8; 4],
    pub level: [u8; 4],
}

#[derive(Debug, Default)]
pub struct KeyboardScaling {
    pub left_depth: u8,
    pub right_depth: u8,
    pub left_curve: u8,
    pub right_curve: u8,
    pub break_point: u8,
}

#[derive(Debug, Default)]
pub struct Operator {
    pub envelope: Envelope,
    pub keyboard_scaling: KeyboardScaling,

    pub rate_scaling: u8,
    pub amp_mod_sensitivity: u8,
    pub velocity_sensitivity: u8,
    pub level: u8,

    pub mode: u8,
    pub coarse: u8,
    pub fine: u8, // x frequency by 1 + 0.01 x fine
    pub detune: u8,
}

#[derive(Debug, Default)]
pub struct ModulationParameters {
    pub delay: u8,
    pub rate: u8,
    pub pitch_mod_depth: u8,
    pub amp_mod_depth: u8,
    pub reset_phase: u8,
    pub waveform: u8,
    pub pitch_mod_sensitivity: u8,
}
