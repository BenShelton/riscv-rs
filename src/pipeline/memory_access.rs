use crate::system_interface::{MMIODevice, SystemInterface};

use super::{LatchValue, PipelineStage, execute::ExecutionValue};

#[derive(Debug, PartialEq, Eq)]
pub struct MemoryAccessValue {
    pub alu_result: u32,
    pub rd: u8,
    pub is_alu_operation: bool,
}

const WIDTH_BYTE: u8 = 0b000;
const WIDTH_HALF: u8 = 0b001;
const WIDTH_WORD: u8 = 0b010;

pub struct InstructionMemoryAccess {
    alu_result: LatchValue<u32>,
    rd: LatchValue<u8>,
    is_alu_operation: LatchValue<bool>,
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
            rd: LatchValue::new(0),
            is_alu_operation: LatchValue::new(false),
        }
    }

    pub fn get_memory_access_value_out(&self) -> MemoryAccessValue {
        MemoryAccessValue {
            alu_result: *self.alu_result.get(),
            rd: *self.rd.get(),
            is_alu_operation: *self.is_alu_operation.get(),
        }
    }
}

impl PipelineStage<InstructionMemoryAccessParams<'_>> for InstructionMemoryAccess {
    fn compute(&mut self, params: InstructionMemoryAccessParams) {
        if params.should_stall {
            return;
        }
        let execution_value = params.execution_value_in;
        self.alu_result.set(execution_value.alu_result);
        self.rd.set(execution_value.rd);
        self.is_alu_operation.set(execution_value.is_alu_operation);

        if execution_value.is_store_operation {
            let addr = execution_value.imm32 + execution_value.rs1;
            match execution_value.funct3 {
                WIDTH_BYTE => {
                    params.bus.write_byte(addr, execution_value.rs2 as u8);
                }
                WIDTH_HALF => {
                    params.bus.write_half_word(addr, execution_value.rs2 as u16);
                }
                WIDTH_WORD => {
                    params.bus.write_word(addr, execution_value.rs2);
                }
                _ => {
                    panic!("Invalid funct3 for store operation");
                }
            }
        }
    }

    fn latch_next(&mut self) {
        self.alu_result.latch_next();
        self.rd.latch_next();
        self.is_alu_operation.latch_next();
    }
}
