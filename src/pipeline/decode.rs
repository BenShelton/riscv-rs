use super::{LatchValue, PipelineStage};
use crate::RegisterFile;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DecodedInstruction {
    None,
    Alu {
        opcode: u8,
        funct3: u8,
        shamt: u8,
        imm11_0: u16,
        rd: u8,
        rs1: u32,
        rs2: u32,
        imm32: u32,
    },
    Store {
        funct3: u8,
        rs1: u32,
        rs2: u32,
        imm32: u32,
    },
}

pub struct InstructionDecode {
    instruction: LatchValue<DecodedInstruction>,
}

pub struct InstructionDecodeParams<'a> {
    pub should_stall: bool,
    pub instruction_in: u32,
    pub reg_file: &'a mut RegisterFile,
}

impl InstructionDecode {
    pub fn new() -> Self {
        Self {
            instruction: LatchValue::new(DecodedInstruction::None),
        }
    }

    pub fn get_decoded_instruction_out(&self) -> DecodedInstruction {
        *self.instruction.get()
    }
}

impl<'a> PipelineStage<InstructionDecodeParams<'a>> for InstructionDecode {
    fn compute(&mut self, params: InstructionDecodeParams<'a>) {
        if params.should_stall {
            return;
        }
        let instruction = params.instruction_in;

        let opcode = (instruction & 0x7F) as u8;
        match opcode {
            0b001_0011 | 0b011_0011 => {
                let imm11_0 = ((instruction >> 20) & 0xFFF) as u16;
                let rs1_address = ((instruction >> 15) & 0x1F) as u8;
                let rs2_address = ((instruction >> 20) & 0x1F) as u8;
                self.instruction.set(DecodedInstruction::Alu {
                    opcode,
                    funct3: ((instruction >> 12) & 0x07) as u8,
                    shamt: rs2_address,
                    imm11_0,
                    rd: ((instruction >> 7) & 0x1F) as u8,
                    rs1: match rs1_address == 0 {
                        true => 0,
                        false => params.reg_file[rs1_address as usize],
                    },
                    rs2: match rs2_address == 0 {
                        true => 0,
                        false => params.reg_file[rs2_address as usize],
                    },
                    imm32: imm11_0 as u32,
                });
            }
            0b010_0011 => {
                let rs1_address = ((instruction >> 15) & 0x1F) as u8;
                let rs2_address = ((instruction >> 20) & 0x1F) as u8;
                self.instruction.set(DecodedInstruction::Store {
                    funct3: ((instruction >> 12) & 0x07) as u8,
                    rs1: match rs1_address == 0 {
                        true => 0,
                        false => params.reg_file[rs1_address as usize],
                    },
                    rs2: match rs2_address == 0 {
                        true => 0,
                        false => params.reg_file[rs2_address as usize],
                    },
                    imm32: (((instruction >> 25) & 0x7F) << 5) | ((instruction >> 7) & 0x1F),
                });
            }
            _ => {
                self.instruction.set(DecodedInstruction::None);
            }
        }
    }

    fn latch_next(&mut self) {
        self.instruction.latch_next();
    }
}
