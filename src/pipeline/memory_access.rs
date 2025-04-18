use crate::{
    system_interface::{MMIODevice, SystemInterface},
    utils::sign_extend_32,
};

use super::{LatchValue, PipelineStage, decode::DecodedInstruction, execute::ExecutionValue};

#[derive(Debug, PartialEq, Eq)]
pub struct MemoryAccessValue {
    pub write_back_value: u32,
    pub instruction: DecodedInstruction,
}

const WIDTH_BYTE: u8 = 0b000;
const WIDTH_HALF: u8 = 0b001;
const WIDTH_WORD: u8 = 0b010;

pub struct InstructionMemoryAccess {
    write_back_value: LatchValue<u32>,
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
            write_back_value: LatchValue::new(0),
            instruction: LatchValue::new(DecodedInstruction::None),
        }
    }

    pub fn get_memory_access_value_out(&self) -> MemoryAccessValue {
        MemoryAccessValue {
            write_back_value: *self.write_back_value.get(),
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

        match execution_value.instruction {
            DecodedInstruction::Alu { .. } => {
                self.write_back_value.set(execution_value.write_back_value);
            }
            DecodedInstruction::Load {
                funct3, imm32, rs1, ..
            } => {
                let addr = (imm32 + rs1 as i32) as u32;
                let should_sign_extend = funct3 & 0b100 == 0;
                self.write_back_value.set(match funct3 & 0b011 {
                    WIDTH_BYTE => {
                        let v = params.bus.read_byte(addr);
                        if should_sign_extend {
                            sign_extend_32(8, v as i32) as u32
                        } else {
                            v as u32
                        }
                    }
                    WIDTH_HALF => {
                        let v = params.bus.read_half_word(addr);
                        if should_sign_extend {
                            sign_extend_32(16, v as i32) as u32
                        } else {
                            v as u32
                        }
                    }
                    WIDTH_WORD => params.bus.read_word(addr),
                    _ => {
                        panic!("Invalid funct3 for load operation");
                    }
                });
            }
            DecodedInstruction::Store {
                funct3,
                imm32,
                rs1,
                rs2,
            } => {
                let addr = (imm32 + rs1 as i32) as u32;
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
            DecodedInstruction::Lui { imm32, .. } => {
                self.write_back_value.set(imm32);
            }
            DecodedInstruction::None => {
                self.write_back_value.set(0);
            }
        }
    }

    fn latch_next(&mut self) {
        self.write_back_value.latch_next();
        self.instruction.latch_next();
    }
}
