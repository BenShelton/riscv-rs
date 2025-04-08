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
    should_stall: Box<dyn Fn() -> bool>,
    get_instruction_in: Box<dyn Fn() -> u32>,
    reg_file: RegisterFile,
}

pub struct InstructionDecodeParams {
    pub should_stall: Box<dyn Fn() -> bool>,
    pub get_instruction_in: Box<dyn Fn() -> u32>,
    pub reg_file: RegisterFile,
}

impl InstructionDecode {
    pub fn new(params: InstructionDecodeParams) -> Self {
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
            should_stall: params.should_stall,
            get_instruction_in: params.get_instruction_in,
            reg_file: params.reg_file,
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
        }
    }
}

impl PipelineStage for InstructionDecode {
    fn compute(&mut self) {
        if (self.should_stall)() {
            return;
        }
        let instruction = (self.get_instruction_in)();
        self.instruction.set(instruction);
        self.opcode.set((instruction & 0x7F) as u8);
        self.rd.set(((instruction >> 7) & 0x1F) as u8);
        self.funct3.set(((instruction >> 12) & 0x07) as u8);
        self.imm11_0.set(((instruction >> 20) & 0xFFF) as u16);
        self.funct7.set(((instruction >> 25) & 0x7F) as u8);
        let rs1_address = ((instruction >> 15) & 0x1F) as u8;
        let rs2_address = ((instruction >> 20) & 0x1F) as u8;
        self.shamt.set(rs2_address);
        self.rs1.set(match rs1_address == 0 {
            true => 0,
            false => self.reg_file.borrow()[rs1_address as usize],
        });
        self.rs2.set(match rs2_address == 0 {
            true => 0,
            false => self.reg_file.borrow()[rs2_address as usize],
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
    }
}
