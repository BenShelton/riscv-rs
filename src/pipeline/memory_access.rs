use crate::system_interface::{MMIODevice, SystemInterface};

use super::{LatchValue, PipelineStage, decode::DecodedInstruction, execute::ExecutionValue};

#[derive(Debug, PartialEq, Eq)]
pub struct MemoryAccessValue {
    pub alu_result: u32,
    pub instruction: DecodedInstruction,
}

const WIDTH_BYTE: u8 = 0b000;
const WIDTH_HALF: u8 = 0b001;
const WIDTH_WORD: u8 = 0b010;

pub struct InstructionMemoryAccess {
    alu_result: LatchValue<u32>,
    instruction: LatchValue<DecodedInstruction>,
}

pub struct InstructionMemoryAccessParams<'a> {
    pub should_stall: bool,
    pub execution_value_in: ExecutionValue,
    pub bus: &'a mut SystemInterface,
}

impl InstructionMemoryAccess {
    pub fn new() -> Self {
        Self {
            alu_result: LatchValue::new(0),
            instruction: LatchValue::new(DecodedInstruction::None),
        }
    }

    pub fn get_memory_access_value_out(&self) -> MemoryAccessValue {
        MemoryAccessValue {
            alu_result: *self.alu_result.get(),
            instruction: *self.instruction.get(),
        }
    }
}

impl PipelineStage<InstructionMemoryAccessParams<'_>> for InstructionMemoryAccess {
    fn compute(&mut self, params: InstructionMemoryAccessParams) {
        if params.should_stall {
            return;
        }
        let execution_value = params.execution_value_in;
        self.instruction.set(execution_value.instruction);
        self.alu_result.set(execution_value.alu_result);

        match execution_value.instruction {
            DecodedInstruction::Alu { .. } => {
                // ALU operations do not require memory access
            }
            DecodedInstruction::Store {
                funct3,
                imm32,
                rs1,
                rs2,
            } => {
                let addr = imm32 + rs1;
                match funct3 {
                    WIDTH_BYTE => {
                        params.bus.write_byte(addr, rs2 as u8);
                    }
                    WIDTH_HALF => {
                        params.bus.write_half_word(addr, rs2 as u16);
                    }
                    WIDTH_WORD => {
                        params.bus.write_word(addr, rs2);
                    }
                    _ => {
                        panic!("Invalid funct3 for store operation");
                    }
                }
            }
            DecodedInstruction::None => {}
        }
    }

    fn latch_next(&mut self) {
        self.alu_result.latch_next();
        self.instruction.latch_next();
    }
}
