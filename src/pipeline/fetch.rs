use super::PipelineStage;
use crate::{
    system_interface::{MMIODevice, PROGRAM_ROM_START, SystemInterface},
    utils::LatchValue,
};

#[derive(Debug, PartialEq, Eq)]
pub struct InstructionValue {
    pub pc: u32,
    pub pc_plus_4: u32,
    pub raw_instruction: u32,
}

pub struct InstructionFetch {
    pub pc: LatchValue<u32>,
    pub pc_plus_4: LatchValue<u32>,
    raw_instruction: LatchValue<u32>,
}

pub struct InstructionFetchParams<'a> {
    pub should_stall: bool,
    pub branch_address: Option<u32>,
    pub bus: &'a SystemInterface,
}

impl InstructionFetch {
    pub fn new() -> Self {
        Self {
            pc: LatchValue::new(PROGRAM_ROM_START),
            pc_plus_4: LatchValue::new(PROGRAM_ROM_START),
            raw_instruction: LatchValue::new(0x0000_0000),
        }
    }

    pub fn get_instruction_value_out(&self) -> InstructionValue {
        InstructionValue {
            pc: *self.pc.get(),
            pc_plus_4: *self.pc_plus_4.get(),
            raw_instruction: *self.raw_instruction.get(),
        }
    }
}

impl<'a> PipelineStage<InstructionFetchParams<'a>> for InstructionFetch {
    fn compute(&mut self, params: InstructionFetchParams<'a>) {
        if params.should_stall {
            return;
        }
        let next_address = match params.branch_address {
            Some(branch_address) => branch_address,
            None => *self.pc_plus_4.get(),
        };
        let value = match params.bus.read_word(next_address) {
            Ok(instruction) => instruction,
            Err(e) => {
                panic!("{}", e);
            }
        };
        self.raw_instruction.set(value);
        self.pc.set(next_address);
        self.pc_plus_4.set(next_address.wrapping_add(4));
    }

    fn latch_next(&mut self) {
        self.raw_instruction.latch_next();
        self.pc.latch_next();
        self.pc_plus_4.latch_next();
    }

    fn reset(&mut self) {
        self.raw_instruction.reset();
        self.pc.reset();
        self.pc_plus_4.reset();
    }
}
