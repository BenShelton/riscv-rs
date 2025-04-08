use super::{LatchValue, PipelineStage};
use crate::system_interface::{MMIODevice, PROGRAM_ROM_START, SystemInterface};
use std::{cell::RefCell, rc::Rc};

pub struct InstructionFetch {
    pc: LatchValue<u32>,
    instruction: LatchValue<u32>,
    bus: Rc<RefCell<SystemInterface>>,
    should_stall: Box<dyn Fn() -> bool>,
}

pub struct InstructionFetchParams {
    pub bus: Rc<RefCell<SystemInterface>>,
    pub should_stall: Box<dyn Fn() -> bool>,
}

impl InstructionFetch {
    pub fn new(params: InstructionFetchParams) -> Self {
        Self {
            pc: LatchValue::new(PROGRAM_ROM_START),
            instruction: LatchValue::new(0x0000_0000),
            bus: params.bus,
            should_stall: params.should_stall,
        }
    }

    pub fn get_instruction_out(&self) -> u32 {
        *self.instruction.get()
    }
}

impl PipelineStage for InstructionFetch {
    fn compute(&mut self) {
        if (self.should_stall)() {
            return;
        }
        self.instruction.set(self.bus.borrow().read(*self.pc.get()));
        self.pc.set(self.pc.get().wrapping_add(4));
    }

    fn latch_next(&mut self) {
        self.instruction.latch_next();
        self.pc.latch_next();
    }
}
