use super::{PipelineStage, decode::DecodedInstruction};

#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionValue {
    pub alu_result: i32,
    pub rd: u8,
    pub is_alu_operation: bool,
}

const ALU_OPERATION_ADD: u8 = 0b000;
const ALU_OPERATION_SLL: u8 = 0b001;
const ALU_OPERATION_SLT: u8 = 0b010;
const ALU_OPERATION_XOR: u8 = 0b100;
const ALU_OPERATION_SRL: u8 = 0b101;
const ALU_OPERATION_OR: u8 = 0b110;
const ALU_OPERATION_AND: u8 = 0b111;

pub struct InstructionExecute {
    alu_result: i32,
    alu_result_next: i32,
    rd: u8,
    rd_next: u8,
    is_alu_operation: bool,
    is_alu_operation_next: bool,
    should_stall: Box<dyn Fn() -> bool>,
    get_decoded_instruction_in: Box<dyn Fn() -> DecodedInstruction>,
}

pub struct InstructionExecuteParams {
    pub should_stall: Box<dyn Fn() -> bool>,
    pub get_decoded_instruction_in: Box<dyn Fn() -> DecodedInstruction>,
}

impl InstructionExecute {
    pub fn new(params: InstructionExecuteParams) -> Self {
        Self {
            alu_result: 0,
            alu_result_next: 0,
            rd: 0,
            rd_next: 0,
            is_alu_operation: false,
            is_alu_operation_next: false,
            should_stall: params.should_stall,
            get_decoded_instruction_in: params.get_decoded_instruction_in,
        }
    }

    pub fn get_execution_value_out(&self) -> ExecutionValue {
        ExecutionValue {
            alu_result: self.alu_result,
            rd: self.rd,
            is_alu_operation: self.is_alu_operation,
        }
    }
}

impl PipelineStage for InstructionExecute {
    fn compute(&mut self) {
        if !(self.should_stall)() {
            let decoded = (self.get_decoded_instruction_in)();
            self.rd_next = decoded.rd;

            let is_register_op = ((decoded.opcode >> 5) & 1) == 1;
            let is_alternate = ((decoded.imm11_0 >> 10) & 1) == 1;
            let imm_32 = decoded.imm11_0 as i32;

            self.is_alu_operation_next = (decoded.opcode & 0b101_1111) == 0b001_0011;

            self.alu_result_next = match decoded.funct3 {
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
                _ => 0,
            };
        }
    }

    fn latch_next(&mut self) {
        self.alu_result = self.alu_result_next;
        self.rd = self.rd_next;
        self.is_alu_operation = self.is_alu_operation_next;
    }
}
