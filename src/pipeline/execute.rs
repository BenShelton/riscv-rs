use super::{LatchValue, PipelineStage, decode::DecodedInstruction};

#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionValue {
    pub alu_result: u32,
    pub instruction: DecodedInstruction,
}

const ALU_OPERATION_ADD: u8 = 0b000;
const ALU_OPERATION_SLL: u8 = 0b001;
const ALU_OPERATION_SLT: u8 = 0b010;
const ALU_OPERATION_SLTU: u8 = 0b011;
const ALU_OPERATION_XOR: u8 = 0b100;
const ALU_OPERATION_SR: u8 = 0b101;
const ALU_OPERATION_OR: u8 = 0b110;
const ALU_OPERATION_AND: u8 = 0b111;

pub struct InstructionExecute {
    alu_result: LatchValue<u32>,
    instruction: LatchValue<DecodedInstruction>,
}

pub struct InstructionExecuteParams {
    pub should_stall: bool,
    pub decoded_instruction_in: DecodedInstruction,
}

impl InstructionExecute {
    pub fn new() -> Self {
        Self {
            alu_result: LatchValue::new(0),
            instruction: LatchValue::new(DecodedInstruction::None),
        }
    }

    pub fn get_execution_value_out(&self) -> ExecutionValue {
        ExecutionValue {
            alu_result: *self.alu_result.get(),
            instruction: *self.instruction.get(),
        }
    }
}

impl PipelineStage<InstructionExecuteParams> for InstructionExecute {
    fn compute(&mut self, params: InstructionExecuteParams) {
        if params.should_stall {
            return;
        }
        let decoded = params.decoded_instruction_in;
        self.instruction.set(decoded);
        match decoded {
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

                self.alu_result.set(match funct3 {
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
            _ => {
                self.alu_result.set(0);
            }
        }
    }

    fn latch_next(&mut self) {
        self.alu_result.latch_next();
        self.instruction.latch_next();
    }
}
