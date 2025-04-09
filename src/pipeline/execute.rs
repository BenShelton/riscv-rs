use super::{LatchValue, PipelineStage, decode::DecodedInstruction};

#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionValue {
    pub alu_result: u32,
    pub rd: u8,
    pub is_alu_operation: bool,
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
    rd: LatchValue<u8>,
    is_alu_operation: LatchValue<bool>,
}

pub struct InstructionExecuteParams {
    pub should_stall: bool,
    pub decoded_instruction_in: DecodedInstruction,
}

impl InstructionExecute {
    pub fn new() -> Self {
        Self {
            alu_result: LatchValue::new(0),
            rd: LatchValue::new(0),
            is_alu_operation: LatchValue::new(false),
        }
    }

    pub fn get_execution_value_out(&self) -> ExecutionValue {
        ExecutionValue {
            alu_result: *self.alu_result.get(),
            rd: *self.rd.get(),
            is_alu_operation: *self.is_alu_operation.get(),
        }
    }
}

impl PipelineStage<InstructionExecuteParams> for InstructionExecute {
    fn compute(&mut self, params: InstructionExecuteParams) {
        if params.should_stall {
            return;
        }
        let decoded = params.decoded_instruction_in;
        self.rd.set(decoded.rd);

        let is_register_op = ((decoded.opcode >> 5) & 1) == 1;
        let is_alternate = ((decoded.imm11_0 >> 10) & 1) == 1;
        let imm_32 = decoded.imm11_0 as u32;

        self.is_alu_operation
            .set((decoded.opcode & 0b101_1111) == 0b001_0011);

        self.alu_result.set(match decoded.funct3 {
            ALU_OPERATION_ADD => {
                if is_register_op {
                    if is_alternate {
                        decoded.rs1 - decoded.rs2
                    } else {
                        decoded.rs1 + decoded.rs2
                    }
                } else {
                    decoded.rs1 + imm_32
                }
            }
            ALU_OPERATION_SLL => {
                if is_register_op {
                    decoded.rs1 << decoded.rs2
                } else {
                    decoded.rs1 << decoded.shamt
                }
            }
            ALU_OPERATION_SLT => {
                if is_register_op {
                    ((decoded.rs1 as i32) < (decoded.rs2 as i32)).into()
                } else {
                    ((decoded.rs1 as i32) < (imm_32 as i32)).into()
                }
            }
            ALU_OPERATION_SLTU => {
                if is_register_op {
                    (decoded.rs1 < decoded.rs2).into()
                } else {
                    (decoded.rs1 < imm_32).into()
                }
            }
            ALU_OPERATION_XOR => {
                if is_register_op {
                    decoded.rs1 ^ decoded.rs2
                } else {
                    decoded.rs1 ^ imm_32
                }
            }
            ALU_OPERATION_SR => {
                if is_register_op {
                    if is_alternate {
                        ((decoded.rs1 as i32) >> (decoded.rs2 as i32)) as u32
                    } else {
                        decoded.rs1 >> decoded.rs2
                    }
                } else {
                    decoded.rs1 >> decoded.shamt
                }
            }
            ALU_OPERATION_OR => {
                if is_register_op {
                    decoded.rs1 | decoded.rs2
                } else {
                    decoded.rs1 | imm_32
                }
            }
            ALU_OPERATION_AND => {
                if is_register_op {
                    decoded.rs1 & decoded.rs2
                } else {
                    decoded.rs1 & imm_32
                }
            }
            _ => 0,
        });
    }

    fn latch_next(&mut self) {
        self.alu_result.latch_next();
        self.rd.latch_next();
        self.is_alu_operation.latch_next();
    }
}
