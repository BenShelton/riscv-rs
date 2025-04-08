use super::PipelineStage;
use crate::RegisterFile;

pub struct DecodedInstruction {
    pub instruction: u32,
    pub opcode: u8,
    pub rd: u8,
    pub funct3: u8,
    pub rs1: i32,
    pub rs2: i32,
    pub imm11_0: u16,
    pub funct7: u8,
    pub shamt: u8,
}

pub struct InstructionDecode {
    instruction: u32,
    instruction_next: u32,
    opcode: u8,
    opcode_next: u8,
    rd: u8,
    rd_next: u8,
    funct3: u8,
    funct3_next: u8,
    rs1: i32,
    rs1_next: i32,
    rs2: i32,
    rs2_next: i32,
    imm11_0: u16,
    imm11_0_next: u16,
    funct7: u8,
    funct7_next: u8,
    shamt: u8,
    shamt_next: u8,
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
            instruction: 0,
            instruction_next: 0,
            opcode: 0,
            opcode_next: 0,
            rd: 0,
            rd_next: 0,
            funct3: 0,
            funct3_next: 0,
            rs1: 0,
            rs1_next: 0,
            rs2: 0,
            rs2_next: 0,
            imm11_0: 0,
            imm11_0_next: 0,
            funct7: 0,
            funct7_next: 0,
            shamt: 0,
            shamt_next: 0,
            should_stall: params.should_stall,
            get_instruction_in: params.get_instruction_in,
            reg_file: params.reg_file,
        }
    }

    pub fn get_decoded_instruction_out(&self) -> DecodedInstruction {
        DecodedInstruction {
            instruction: self.instruction,
            opcode: self.opcode,
            rd: self.rd,
            funct3: self.funct3,
            rs1: self.rs1,
            rs2: self.rs2,
            imm11_0: self.imm11_0,
            funct7: self.funct7,
            shamt: self.shamt,
        }
    }
}

impl PipelineStage for InstructionDecode {
    fn compute(&mut self) {
        if (self.should_stall)() {
            return;
        }
        self.instruction_next = (self.get_instruction_in)();
        self.opcode_next = (self.instruction_next & 0x7F) as u8;
        self.rd_next = ((self.instruction_next >> 7) & 0x1F) as u8;
        self.funct3_next = ((self.instruction_next >> 12) & 0x07) as u8;
        self.imm11_0_next = ((self.instruction_next >> 20) & 0x7FF) as u16;
        self.funct7_next = ((self.instruction_next >> 25) & 0x7F) as u8;
        let rs1_address = ((self.instruction_next >> 15) & 0x1F) as u8;
        let rs2_address = ((self.instruction_next >> 20) & 0x1F) as u8;
        self.shamt_next = rs2_address;
        self.rs1_next = match rs1_address == 0 {
            true => 0,
            false => self.reg_file.borrow()[rs1_address as usize],
        };
        self.rs2_next = match rs2_address == 0 {
            true => 0,
            false => self.reg_file.borrow()[rs2_address as usize],
        };
    }

    fn latch_next(&mut self) {
        self.instruction = self.instruction_next;
        self.opcode = self.opcode_next;
        self.rd = self.rd_next;
        self.funct3 = self.funct3_next;
        self.rs1 = self.rs1_next;
        self.rs2 = self.rs2_next;
        self.imm11_0 = self.imm11_0_next;
        self.funct7 = self.funct7_next;
        self.shamt = self.shamt_next;
    }
}
