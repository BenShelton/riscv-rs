use crate::utils::LatchValue;

use super::{
    PipelineStage,
    decode::{DecodedInstruction, DecodedValue},
};

#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionValue {
    pub write_back_value: u32,
    pub instruction: DecodedInstruction,
    pub raw_instruction: u32,
    pub pc: u32,
    pub pc_plus_4: u32,
}

const ALU_OPERATION_ADD: u8 = 0b000;
const ALU_OPERATION_SLL: u8 = 0b001;
const ALU_OPERATION_SLT: u8 = 0b010;
const ALU_OPERATION_SLTU: u8 = 0b011;
const ALU_OPERATION_XOR: u8 = 0b100;
const ALU_OPERATION_SR: u8 = 0b101;
const ALU_OPERATION_OR: u8 = 0b110;
const ALU_OPERATION_AND: u8 = 0b111;

const BRANCH_OPERATION_EQ: u8 = 0b000;
const BRANCH_OPERATION_NE: u8 = 0b001;
const BRANCH_OPERATION_LT: u8 = 0b100;
const BRANCH_OPERATION_GE: u8 = 0b101;
const BRANCH_OPERATION_LTU: u8 = 0b110;
const BRANCH_OPERATION_GEU: u8 = 0b111;

pub struct InstructionExecute {
    write_back_value: LatchValue<u32>,
    instruction: LatchValue<DecodedInstruction>,
    raw_instruction: LatchValue<u32>,
    pc: LatchValue<u32>,
    pc_plus_4: LatchValue<u32>,
}

pub struct InstructionExecuteParams {
    pub should_stall: bool,
    pub decoded_instruction_in: DecodedValue,
}

impl InstructionExecute {
    pub fn new() -> Self {
        Self {
            write_back_value: LatchValue::new(0),
            instruction: LatchValue::new(DecodedInstruction::None),
            raw_instruction: LatchValue::new(0),
            pc: LatchValue::new(0),
            pc_plus_4: LatchValue::new(0),
        }
    }

    pub fn get_execution_value_out(&self) -> ExecutionValue {
        ExecutionValue {
            write_back_value: *self.write_back_value.get(),
            instruction: *self.instruction.get(),
            raw_instruction: *self.raw_instruction.get(),
            pc: *self.pc.get(),
            pc_plus_4: *self.pc_plus_4.get(),
        }
    }
}

impl PipelineStage<InstructionExecuteParams> for InstructionExecute {
    fn compute(&mut self, params: InstructionExecuteParams) {
        if params.should_stall {
            return;
        }
        let decoded = params.decoded_instruction_in;
        self.instruction.set(decoded.instruction);
        self.raw_instruction.set(decoded.raw_instruction);
        self.pc.set(decoded.pc);
        self.pc_plus_4.set(decoded.pc_plus_4);

        match decoded.instruction {
            DecodedInstruction::Alu {
                opcode,
                funct3,
                shamt,
                imm11_0,
                rs1,
                rs2,
                imm32,
                ..
            } => {
                let is_register_op = ((opcode >> 5) & 1) == 1;
                let is_alternate = ((imm11_0 >> 10) & 1) == 1;

                self.write_back_value.set(match funct3 {
                    ALU_OPERATION_ADD => {
                        if is_register_op {
                            if is_alternate { rs1 - rs2 } else { rs1 + rs2 }
                        } else {
                            (rs1 as i32).saturating_add(imm32) as u32
                        }
                    }
                    ALU_OPERATION_SLL => {
                        if is_register_op {
                            rs1 << rs2
                        } else {
                            rs1 << shamt
                        }
                    }
                    ALU_OPERATION_SLT => {
                        if is_register_op {
                            ((rs1 as i32) < (rs2 as i32)).into()
                        } else {
                            ((rs1 as i32) < imm32).into()
                        }
                    }
                    ALU_OPERATION_SLTU => {
                        if is_register_op {
                            (rs1 < rs2).into()
                        } else {
                            (rs1 < (imm32 as u32)).into()
                        }
                    }
                    ALU_OPERATION_XOR => {
                        if is_register_op {
                            rs1 ^ rs2
                        } else {
                            rs1 ^ (imm32 as u32)
                        }
                    }
                    ALU_OPERATION_SR => {
                        if is_register_op {
                            if is_alternate {
                                ((rs1 as i32) >> (rs2 as i32)) as u32
                            } else {
                                rs1 >> rs2
                            }
                        } else {
                            rs1 >> shamt
                        }
                    }
                    ALU_OPERATION_OR => {
                        if is_register_op {
                            rs1 | rs2
                        } else {
                            rs1 | (imm32 as u32)
                        }
                    }
                    ALU_OPERATION_AND => {
                        if is_register_op {
                            rs1 & rs2
                        } else {
                            rs1 & (imm32 as u32)
                        }
                    }
                    _ => 0,
                });
            }
            DecodedInstruction::Branch {
                funct3, rs1, rs2, ..
            } => {
                let branch_taken = match funct3 {
                    BRANCH_OPERATION_EQ => rs1 == rs2,
                    BRANCH_OPERATION_NE => rs1 != rs2,
                    BRANCH_OPERATION_LT => (rs1 as i32) < (rs2 as i32),
                    BRANCH_OPERATION_GE => (rs1 as i32) >= (rs2 as i32),
                    BRANCH_OPERATION_LTU => rs1 < rs2,
                    BRANCH_OPERATION_GEU => rs1 >= rs2,
                    _ => false,
                };
                if !branch_taken {
                    self.instruction.set(DecodedInstruction::Branch {
                        funct3,
                        branch_address: decoded.pc_plus_4,
                        rs1,
                        rs2,
                    });
                }
                self.write_back_value.set(0);
            }
            _ => {
                self.write_back_value.set(0);
            }
        }
    }

    fn latch_next(&mut self) {
        self.write_back_value.latch_next();
        self.instruction.latch_next();
        self.raw_instruction.latch_next();
        self.pc.latch_next();
        self.pc_plus_4.latch_next();
    }
}
