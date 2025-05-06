use crate::{
    csr::{CSR_OPERATION_RC, CSR_OPERATION_RS, CSR_OPERATION_RW, CSRInterface},
    system_interface::{MMIODevice, SystemInterface},
    utils::{LatchValue, sign_extend_32},
};

use super::{PipelineStage, decode::DecodedInstruction, execute::ExecutionValue};

#[derive(Debug, PartialEq, Eq)]
pub struct MemoryAccessValue {
    pub write_back_value: u32,
    pub pc: u32,
    pub pc_plus_4: u32,
    pub instruction: DecodedInstruction,
}

const WIDTH_BYTE: u8 = 0b000;
const WIDTH_HALF: u8 = 0b001;
const WIDTH_WORD: u8 = 0b010;

pub struct InstructionMemoryAccess {
    write_back_value: LatchValue<u32>,
    pc: LatchValue<u32>,
    pc_plus_4: LatchValue<u32>,
    instruction: LatchValue<DecodedInstruction>,
}

pub struct InstructionMemoryAccessParams<'a> {
    pub should_stall: bool,
    pub execution_value_in: ExecutionValue,
    pub bus: &'a mut SystemInterface,
    pub csr: &'a mut CSRInterface,
}

impl InstructionMemoryAccess {
    pub fn new() -> Self {
        Self {
            write_back_value: LatchValue::new(0),
            pc: LatchValue::new(0),
            pc_plus_4: LatchValue::new(0),
            instruction: LatchValue::new(DecodedInstruction::None),
        }
    }

    pub fn get_memory_access_value_out(&self) -> MemoryAccessValue {
        MemoryAccessValue {
            write_back_value: *self.write_back_value.get(),
            instruction: *self.instruction.get(),
            pc: *self.pc.get(),
            pc_plus_4: *self.pc_plus_4.get(),
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
        self.pc.set(execution_value.pc);
        self.pc_plus_4.set(execution_value.pc_plus_4);

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
                        let v = params.bus.read_byte(addr).unwrap();
                        if should_sign_extend {
                            sign_extend_32(8, v as i32) as u32
                        } else {
                            v as u32
                        }
                    }
                    WIDTH_HALF => {
                        let v = params.bus.read_half_word(addr).unwrap();
                        if should_sign_extend {
                            sign_extend_32(16, v as i32) as u32
                        } else {
                            v as u32
                        }
                    }
                    WIDTH_WORD => params.bus.read_word(addr).unwrap(),
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
                        params.bus.write_byte(addr, rs2 as u8).unwrap();
                    }
                    WIDTH_HALF => {
                        params.bus.write_half_word(addr, rs2 as u16).unwrap();
                    }
                    WIDTH_WORD => {
                        params.bus.write_word(addr, rs2).unwrap();
                    }
                    _ => {
                        panic!("Invalid funct3 for store operation");
                    }
                }
            }
            DecodedInstruction::Lui { imm32, .. } => {
                self.write_back_value.set(imm32);
            }
            DecodedInstruction::Jal { .. } => {
                self.write_back_value.set(execution_value.pc_plus_4);
            }
            DecodedInstruction::Branch { .. } => {
                self.write_back_value.set(0);
            }
            DecodedInstruction::System {
                funct3,
                csr_address,
                source,
                should_write,
                should_read,
                ..
            } => {
                let csr_value = should_read
                    .then(|| params.csr.read(csr_address))
                    .unwrap_or(0);
                self.write_back_value.set(csr_value);

                if should_write {
                    match funct3 & 0b11 {
                        CSR_OPERATION_RW => {
                            params.csr.write(csr_address, source);
                        }
                        CSR_OPERATION_RS => {
                            params.csr.write(csr_address, csr_value | source);
                        }
                        CSR_OPERATION_RC => {
                            params.csr.write(csr_address, csr_value & !source);
                        }
                        _ => {}
                    }
                }
            }
            DecodedInstruction::Auipc { imm32, .. } => {
                self.write_back_value.set(execution_value.pc + imm32);
            }
            DecodedInstruction::None => {
                self.write_back_value.set(0);
            }
        }
    }

    fn latch_next(&mut self) {
        self.write_back_value.latch_next();
        self.instruction.latch_next();
        self.pc.latch_next();
        self.pc_plus_4.latch_next();
    }
}
