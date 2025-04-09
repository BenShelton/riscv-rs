use super::{LatchValue, PipelineStage};
use crate::system_interface::{MMIODevice, PROGRAM_ROM_START, SystemInterface};

pub struct InstructionFetch {
    pc: LatchValue<u32>,
    instruction: LatchValue<u32>,
}

pub struct InstructionFetchParams<'a> {
    pub should_stall: bool,
    pub bus: &'a SystemInterface,
}

impl InstructionFetch {
    pub fn new() -> Self {
        Self {
            pc: LatchValue::new(PROGRAM_ROM_START),
            instruction: LatchValue::new(0x0000_0000),
        }
    }

    pub fn get_instruction_out(&self) -> u32 {
        *self.instruction.get()
    }
}

impl<'a> PipelineStage<InstructionFetchParams<'a>> for InstructionFetch {
    fn compute(&mut self, params: InstructionFetchParams<'a>) {
        if params.should_stall {
            return;
        }
        self.instruction.set(params.bus.read(*self.pc.get()));
        self.pc.set(self.pc.get().wrapping_add(4));
    }

    fn latch_next(&mut self) {
        self.instruction.latch_next();
        self.pc.latch_next();
    }
}
