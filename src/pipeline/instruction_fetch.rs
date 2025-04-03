use super::PipelineStage;
use crate::system_interface::{MMIODevice, PROGRAM_ROM_START, SystemInterface};

pub struct InstructionFetch<'a> {
    pc: u32,
    pc_next: u32,
    instruction: u32,
    instruction_next: u32,
    bus: &'a SystemInterface,
    should_stall: fn() -> bool,
}

pub struct InstructionFetchParams<'a> {
    bus: &'a SystemInterface,
    should_stall: fn() -> bool,
}

impl<'a> InstructionFetch<'a> {
    pub fn new(params: InstructionFetchParams<'a>) -> Self {
        Self {
            pc: PROGRAM_ROM_START,
            pc_next: PROGRAM_ROM_START,
            instruction: 0x0000_0000,
            instruction_next: 0x0000_0000,
            bus: params.bus,
            should_stall: params.should_stall,
        }
    }

    fn get_instruction_out(&self) -> u32 {
        self.instruction
    }
}

impl<'a> PipelineStage for InstructionFetch<'a> {
    fn ready_to_send(&self) -> bool {
        true
    }

    fn ready_to_receive(&self) -> bool {
        true
    }

    fn compute(&mut self) {
        if !(self.should_stall)() {
            self.instruction_next = self.bus.read(self.pc);
            self.pc_next = self.pc_next.wrapping_add(4);
        }
    }

    fn latch_next(&mut self) {
        self.instruction = self.instruction_next;
        self.pc = self.pc_next;
    }
}
