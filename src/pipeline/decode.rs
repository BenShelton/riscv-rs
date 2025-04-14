use super::{LatchValue, PipelineStage};
use crate::RegisterFile;

#[derive(Debug, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub instruction: u32,
    pub opcode: u8,
    pub rd: u8,
    pub funct3: u8,
    pub rs1: u32,
    pub rs2: u32,
    pub imm11_0: u16,
    pub funct7: u8,
    pub shamt: u8,
    pub is_alu_operation: bool,
    pub is_store_operation: bool,
    pub imm32: u32,
}

pub struct InstructionDecode {
    instruction: LatchValue<u32>,
    opcode: LatchValue<u8>,
    rd: LatchValue<u8>,
    funct3: LatchValue<u8>,
    rs1: LatchValue<u32>,
    rs2: LatchValue<u32>,
    imm11_0: LatchValue<u16>,
    funct7: LatchValue<u8>,
    shamt: LatchValue<u8>,
    is_alu_operation: LatchValue<bool>,
    is_store_operation: LatchValue<bool>,
    imm32: LatchValue<u32>,
}

pub struct InstructionDecodeParams<'a> {
    pub should_stall: bool,
    pub instruction_in: u32,
    pub reg_file: &'a mut RegisterFile,
}

impl InstructionDecode {
    pub fn new() -> Self {
        Self {
            instruction: LatchValue::new(0),
            opcode: LatchValue::new(0),
            rd: LatchValue::new(0),
            funct3: LatchValue::new(0),
            rs1: LatchValue::new(0),
            rs2: LatchValue::new(0),
            imm11_0: LatchValue::new(0),
            funct7: LatchValue::new(0),
            shamt: LatchValue::new(0),
            is_alu_operation: LatchValue::new(false),
            is_store_operation: LatchValue::new(false),
            imm32: LatchValue::new(0),
        }
    }

    pub fn get_decoded_instruction_out(&self) -> DecodedInstruction {
        DecodedInstruction {
            instruction: *self.instruction.get(),
            opcode: *self.opcode.get(),
            rd: *self.rd.get(),
            funct3: *self.funct3.get(),
            rs1: *self.rs1.get(),
            rs2: *self.rs2.get(),
            imm11_0: *self.imm11_0.get(),
            funct7: *self.funct7.get(),
            shamt: *self.shamt.get(),
            is_alu_operation: *self.is_alu_operation.get(),
            is_store_operation: *self.is_store_operation.get(),
            imm32: *self.imm32.get(),
        }
    }
}

impl<'a> PipelineStage<InstructionDecodeParams<'a>> for InstructionDecode {
    fn compute(&mut self, params: InstructionDecodeParams<'a>) {
        if params.should_stall {
            return;
        }
        let instruction = params.instruction_in;
        self.instruction.set(instruction);

        let opcode = (instruction & 0x7F) as u8;
        self.opcode.set(opcode);
        let is_alu_operation = (opcode & 0b101_1111) == 0b001_0011;
        self.is_alu_operation.set(is_alu_operation);
        let is_store_operation = opcode == 0b010_0011;
        self.is_store_operation.set(is_store_operation);

        let imm11_0 = ((instruction >> 20) & 0xFFF) as u16;
        self.imm11_0.set(imm11_0);
        self.imm32
            .set(match (is_alu_operation, is_store_operation) {
                (_, true) => (((instruction >> 25) & 0x7F) << 5) | ((instruction >> 7) & 0x1F),
                _ => imm11_0 as u32,
            });

        self.rd.set(((instruction >> 7) & 0x1F) as u8);
        self.funct3.set(((instruction >> 12) & 0x07) as u8);
        self.funct7.set(((instruction >> 25) & 0x7F) as u8);
        let rs1_address = ((instruction >> 15) & 0x1F) as u8;
        let rs2_address = ((instruction >> 20) & 0x1F) as u8;
        self.shamt.set(rs2_address);
        self.rs1.set(match rs1_address == 0 {
            true => 0,
            false => params.reg_file[rs1_address as usize],
        });
        self.rs2.set(match rs2_address == 0 {
            true => 0,
            false => params.reg_file[rs2_address as usize],
        });
    }

    fn latch_next(&mut self) {
        self.instruction.latch_next();
        self.opcode.latch_next();
        self.rd.latch_next();
        self.funct3.latch_next();
        self.rs1.latch_next();
        self.rs2.latch_next();
        self.imm11_0.latch_next();
        self.funct7.latch_next();
        self.shamt.latch_next();
        self.is_alu_operation.latch_next();
        self.is_store_operation.latch_next();
        self.imm32.latch_next();
    }
}
