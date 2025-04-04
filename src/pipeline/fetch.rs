use super::PipelineStage;
use crate::system_interface::{MMIODevice, PROGRAM_ROM_START, SystemInterface};
use std::{cell::RefCell, rc::Rc};

pub struct InstructionFetch {
    pc: u32,
    pc_next: u32,
    instruction: u32,
    instruction_next: u32,
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
            pc: PROGRAM_ROM_START,
            pc_next: PROGRAM_ROM_START,
            instruction: 0x0000_0000,
            instruction_next: 0x0000_0000,
            bus: params.bus,
            should_stall: params.should_stall,
        }
    }

    pub fn get_instruction_out(&self) -> u32 {
        self.instruction
    }
}

impl PipelineStage for InstructionFetch {
    fn compute(&mut self) {
        if !(self.should_stall)() {
            self.instruction_next = self.bus.borrow().read(self.pc);
            self.pc_next = self.pc_next.wrapping_add(4);
        }
    }

    fn latch_next(&mut self) {
        self.instruction = self.instruction_next;
        self.pc = self.pc_next;
    }
}
