use super::{LatchValue, PipelineStage, decode::DecodedInstruction};

#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionValue {
    pub alu_result: u32,
    pub rd: u8,
    pub is_alu_operation: bool,
    pub is_store_operation: bool,
    pub imm32: u32,
    pub funct3: u8,
    pub rs1: u32,
    pub rs2: u32,
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
    is_store_operation: LatchValue<bool>,
    imm32: LatchValue<u32>,
    funct3: LatchValue<u8>,
    rs1: LatchValue<u32>,
    rs2: LatchValue<u32>,
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
            is_store_operation: LatchValue::new(false),
            imm32: LatchValue::new(0),
            funct3: LatchValue::new(0),
            rs1: LatchValue::new(0),
            rs2: LatchValue::new(0),
        }
    }

    pub fn get_execution_value_out(&self) -> ExecutionValue {
        ExecutionValue {
            alu_result: *self.alu_result.get(),
            rd: *self.rd.get(),
            is_alu_operation: *self.is_alu_operation.get(),
            is_store_operation: *self.is_store_operation.get(),
            imm32: *self.imm32.get(),
            funct3: *self.funct3.get(),
            rs1: *self.rs1.get(),
            rs2: *self.rs2.get(),
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
        self.is_alu_operation.set(decoded.is_alu_operation);
        self.is_store_operation.set(decoded.is_store_operation);
        self.imm32.set(decoded.imm32);
        self.funct3.set(decoded.funct3);
        self.rs1.set(decoded.rs1);
        self.rs2.set(decoded.rs2);

        if !decoded.is_alu_operation {
            self.alu_result.set(0);
        } else {
            let is_register_op = ((decoded.opcode >> 5) & 1) == 1;
            let is_alternate = ((decoded.imm11_0 >> 10) & 1) == 1;

            self.alu_result.set(match decoded.funct3 {
                ALU_OPERATION_ADD => {
                    if is_register_op {
                        if is_alternate {
                            decoded.rs1 - decoded.rs2
                        } else {
                            decoded.rs1 + decoded.rs2
                        }
                    } else {
                        decoded.rs1 + decoded.imm32
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
                        ((decoded.rs1 as i32) < (decoded.imm32 as i32)).into()
                    }
                }
                ALU_OPERATION_SLTU => {
                    if is_register_op {
                        (decoded.rs1 < decoded.rs2).into()
                    } else {
                        (decoded.rs1 < decoded.imm32).into()
                    }
                }
                ALU_OPERATION_XOR => {
                    if is_register_op {
                        decoded.rs1 ^ decoded.rs2
                    } else {
                        decoded.rs1 ^ decoded.imm32
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
                        decoded.rs1 | decoded.imm32
                    }
                }
                ALU_OPERATION_AND => {
                    if is_register_op {
                        decoded.rs1 & decoded.rs2
                    } else {
                        decoded.rs1 & decoded.imm32
                    }
                }
                _ => 0,
            });
        }
    }

    fn latch_next(&mut self) {
        self.alu_result.latch_next();
        self.rd.latch_next();
        self.is_alu_operation.latch_next();
        self.is_store_operation.latch_next();
        self.imm32.latch_next();
        self.funct3.latch_next();
        self.rs1.latch_next();
        self.rs2.latch_next();
    }
}
