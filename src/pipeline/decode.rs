use super::{PipelineStage, fetch::InstructionValue};
use crate::{
    RegisterFile,
    utils::{LatchValue, bit, sign_extend_32, slice_32},
};

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
        imm32: i32,
    },
    Store {
        funct3: u8,
        rs1: u32,
        rs2: u32,
        imm32: i32,
    },
    Load {
        funct3: u8,
        rd: u8,
        rs1: u32,
        imm32: i32,
    },
    Lui {
        rd: u8,
        imm32: u32,
    },
    Jal {
        rd: u8,
        branch_address: u32,
        pc: u32,
        pc_plus_4: u32,
    },
    Branch {
        funct3: u8,
        branch_address: u32,
        pc: u32,
        pc_plus_4: u32,
        rs1: u32,
        rs2: u32,
    },
    System {
        funct3: u8,
        csr_address: u32,
        rd: u8,
        source: u32,
        should_write: bool,
        should_read: bool,
    },
    Auipc {
        pc: u32,
        rd: u8,
        imm32: u32,
    },
}

pub struct InstructionDecode {
    instruction: LatchValue<DecodedInstruction>,
}

pub struct InstructionDecodeParams<'a> {
    pub should_stall: bool,
    pub instruction_in: InstructionValue,
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
        let instruction = params.instruction_in.instruction;

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
                    imm32: sign_extend_32(12, imm11_0 as i32),
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
                    imm32: sign_extend_32(
                        12,
                        ((((instruction >> 25) & 0x7F) << 5) | ((instruction >> 7) & 0x1F)) as i32,
                    ),
                });
            }
            0b000_0011 => {
                let imm11_0 = ((instruction >> 20) & 0xFFF) as u16;
                let rs1_address = ((instruction >> 15) & 0x1F) as u8;
                self.instruction.set(DecodedInstruction::Load {
                    funct3: ((instruction >> 12) & 0x07) as u8,
                    rd: ((instruction >> 7) & 0x1F) as u8,
                    rs1: match rs1_address == 0 {
                        true => 0,
                        false => params.reg_file[rs1_address as usize],
                    },
                    imm32: sign_extend_32(12, imm11_0 as i32),
                });
            }
            0b0110111 => {
                self.instruction.set(DecodedInstruction::Lui {
                    rd: ((instruction >> 7) & 0x1F) as u8,
                    imm32: (instruction >> 12) << 12,
                });
            }
            0b1101111 => {
                let restructured_imm = bit(31, instruction, 20)
                    | slice_32(19, 12, instruction, 19)
                    | bit(20, instruction, 11)
                    | slice_32(30, 21, instruction, 10);
                let imm32 = sign_extend_32(21, (restructured_imm << 1) as i32);
                self.instruction.set(DecodedInstruction::Jal {
                    rd: ((instruction >> 7) & 0x1F) as u8,
                    branch_address: params.instruction_in.pc.saturating_add_signed(imm32),
                    pc: params.instruction_in.pc,
                    pc_plus_4: params.instruction_in.pc_plus_4,
                });
            }
            0b1100111 => {
                let imm11_0 = ((instruction >> 20) & 0xFFF) as u16;
                let i_imm = sign_extend_32(12, imm11_0 as i32);
                let imm32 = slice_32(11, 1, i_imm as u32, 11);
                let rs1_address = ((instruction >> 15) & 0x1F) as u8;
                let rs1 = match rs1_address == 0 {
                    true => 0,
                    false => params.reg_file[rs1_address as usize],
                };
                self.instruction.set(DecodedInstruction::Jal {
                    rd: ((instruction >> 7) & 0x1F) as u8,
                    branch_address: rs1 + imm32,
                    pc: params.instruction_in.pc,
                    pc_plus_4: params.instruction_in.pc_plus_4,
                });
            }
            0b1100011 => {
                let restructured_imm = bit(31, instruction, 12)
                    | bit(7, instruction, 11)
                    | slice_32(30, 25, instruction, 10)
                    | slice_32(11, 8, instruction, 4);
                let imm32 = sign_extend_32(13, (restructured_imm << 1) as i32);
                let rs1_address = ((instruction >> 15) & 0x1F) as u8;
                let rs2_address = ((instruction >> 20) & 0x1F) as u8;
                self.instruction.set(DecodedInstruction::Branch {
                    funct3: ((instruction >> 12) & 0x07) as u8,
                    branch_address: params.instruction_in.pc.saturating_add_signed(imm32),
                    pc: params.instruction_in.pc,
                    pc_plus_4: params.instruction_in.pc_plus_4,
                    rs1: match rs1_address == 0 {
                        true => 0,
                        false => params.reg_file[rs1_address as usize],
                    },
                    rs2: match rs2_address == 0 {
                        true => 0,
                        false => params.reg_file[rs2_address as usize],
                    },
                });
            }
            0b1110011 => {
                let rd = ((instruction >> 7) & 0x1F) as u8;
                let rs1_address = ((instruction >> 15) & 0x1F) as u8;
                let funct3 = ((instruction >> 12) & 0x07) as u8;
                let source = match funct3 & 0b100 {
                    0b100 => rs1_address as u32,
                    _ => params.reg_file[rs1_address as usize],
                };
                let should_write = match funct3 & 0b11 {
                    0b01 => true,
                    _ => rs1_address != 0,
                };
                let should_read = match funct3 & 0b11 {
                    0b01 => rd != 0,
                    _ => true,
                };

                self.instruction.set(DecodedInstruction::System {
                    funct3,
                    csr_address: instruction >> 20,
                    rd,
                    source,
                    should_write,
                    should_read,
                });
            }
            0b0010111 => {
                self.instruction.set(DecodedInstruction::Auipc {
                    pc: params.instruction_in.pc,
                    rd: ((instruction >> 7) & 0x1F) as u8,
                    imm32: (instruction >> 12) << 12,
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
