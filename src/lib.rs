#![allow(dead_code)]
#![allow(clippy::unusual_byte_groupings)]

mod pipeline;
mod system_interface;

use pipeline::{
    PipelineStage,
    decode::{InstructionDecode, InstructionDecodeParams},
    execute::{InstructionExecute, InstructionExecuteParams},
    fetch::{InstructionFetch, InstructionFetchParams},
    memory_access::{InstructionMemoryAccess, InstructionMemoryAccessParams},
    write_back::{InstructionWriteBack, InstructionWriteBackParams},
};
use system_interface::{RamDevice, RomDevice, SystemInterface};

#[derive(PartialEq, Eq, Debug)]
pub enum State {
    Fetch,
    Decode,
    Execute,
    MemoryAccess,
    WriteBack,
}

pub type RegisterFile = [u32; 32];

pub struct RVI32System {
    pub bus: SystemInterface,
    pub state: State,
    pub reg_file: RegisterFile,
    stage_if: InstructionFetch,
    stage_de: InstructionDecode,
    stage_ex: InstructionExecute,
    stage_ma: InstructionMemoryAccess,
    stage_wb: InstructionWriteBack,
}

impl RVI32System {
    pub fn new() -> Self {
        let rom = RomDevice::new();
        let ram = RamDevice::new();

        Self {
            bus: SystemInterface::new(rom, ram),
            state: State::Fetch,
            reg_file: [0u32; 32],
            stage_if: InstructionFetch::new(),
            stage_de: InstructionDecode::new(),
            stage_ex: InstructionExecute::new(),
            stage_ma: InstructionMemoryAccess::new(),
            stage_wb: InstructionWriteBack::new(),
        }
    }

    pub fn compute(&mut self) {
        self.stage_if.compute(InstructionFetchParams {
            should_stall: self.state != State::Fetch,
            bus: &self.bus,
        });
        self.stage_de.compute(InstructionDecodeParams {
            should_stall: self.state != State::Decode,
            instruction_in: self.stage_if.get_instruction_out(),
            reg_file: &mut self.reg_file,
        });
        self.stage_ex.compute(InstructionExecuteParams {
            should_stall: self.state != State::Execute,
            decoded_instruction_in: self.stage_de.get_decoded_instruction_out(),
        });
        self.stage_ma.compute(InstructionMemoryAccessParams {
            should_stall: self.state != State::MemoryAccess,
            execution_value_in: self.stage_ex.get_execution_value_out(),
        });
        self.stage_wb.compute(InstructionWriteBackParams {
            should_stall: self.state != State::WriteBack,
            memory_access_value_in: self.stage_ma.get_memory_access_value_out(),
            reg_file: &mut self.reg_file,
        });
    }

    pub fn latch_next(&mut self) {
        self.stage_if.latch_next();
        self.stage_de.latch_next();
        self.stage_ex.latch_next();
        self.stage_ma.latch_next();
        self.stage_wb.latch_next();
    }

    pub fn cycle(&mut self) {
        self.compute();
        self.latch_next();

        self.state = match self.state {
            State::Fetch => State::Decode,
            State::Decode => State::Execute,
            State::Execute => State::MemoryAccess,
            State::MemoryAccess => State::WriteBack,
            State::WriteBack => State::Fetch,
        };
    }
}

impl Default for RVI32System {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pipeline::{
            decode::DecodedInstruction, execute::ExecutionValue, memory_access::MemoryAccessValue,
        },
        system_interface::MMIODevice,
    };

    #[test]
    fn test_rom_read() {
        let mut rv = RVI32System::new();
        rv.bus.rom.load(vec![0xDEAD_BEEF, 0xC0DE_CAFE]);
        assert_eq!(rv.bus.read(0x1000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.read(0x1000_0004), 0xC0DE_CAFE);
        assert_eq!(rv.bus.read(0x1000_0008), 0xFFFF_FFFF);
    }

    #[test]
    fn test_rom_write_does_nothing() {
        let mut rv = RVI32System::new();
        rv.bus.write(0x1000_0000, 0xDEAD_BEEF);
        rv.bus.write(0x1000_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.read(0x1000_0000), 0xFFFF_FFFF);
        assert_eq!(rv.bus.read(0x1000_0004), 0xFFFF_FFFF);
    }

    #[test]
    fn test_ram_write_read() {
        let mut rv = RVI32System::new();
        rv.bus.write(0x2000_0000, 0xDEAD_BEEF);
        rv.bus.write(0x2000_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.read(0x2000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.read(0x2000_0004), 0xC0DE_CAFE);
    }

    #[test]
    fn test_ram_write_wrap_around() {
        let mut rv = RVI32System::new();
        rv.bus.write(0x2040_0000, 0xDEAD_BEEF);
        rv.bus.write(0x2040_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.read(0x2000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.read(0x2000_0004), 0xC0DE_CAFE);
    }

    #[test]
    #[should_panic(expected = "Unaligned read from address 0x10000005")]
    fn test_panic_on_misaligned_read() {
        let rv = RVI32System::new();
        rv.bus.read(0x1000_0005);
    }

    #[test]
    #[should_panic(expected = "Unaligned write to address 0x10000005 (value=0xDEADBEEF)")]
    fn test_panic_on_misaligned_write() {
        let mut rv = RVI32System::new();
        rv.bus.write(0x1000_0005, 0xDEAD_BEEF);
    }

    #[test]
    fn test_instructions() {
        let mut rv = RVI32System::new();
        rv.reg_file[1] = 0x0102_0304;
        rv.reg_file[2] = 0x0203_0405;
        rv.reg_file[10] = 0x8000_0000;
        rv.reg_file[11] = 0x0000_0001;

        rv.bus.rom.load(vec![
            0b000000000001_00001_000_00011_0010011,  // ADDI 1, r1, r3
            0b0000000_00001_00010_000_00100_0110011, // ADD r1, r2, r4
            0b0100000_00001_00010_000_00100_0110011, // SUB r1, r2, r4
            0b111111111111_00001_000_00011_0010011,  // ADDI 4095, r1, r3
            0b0000000_01011_01010_101_01100_0110011, // SRL r10, r11, r12
            0b0100000_01011_01010_101_01100_0110011, // SRA r10, r11, r12
        ]);

        // ADDI 1, r1, r3
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_out(),
            0b000000000001_00001_000_00011_0010011
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedInstruction {
                instruction: 0b000000000001_00001_000_00011_0010011,
                opcode: 0b0010011,
                rd: 0b00011,
                funct3: 0b000,
                imm11_0: 0b000000000001,
                rs1: 0x0102_0304,
                rs2: 0x0102_0304,
                funct7: 0b0000000,
                shamt: 0b00001
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                alu_result: 0x0102_0305,
                rd: 0b00011,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                alu_result: 0x0102_0305,
                rd: 0b00011,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b00011], 0x0102_0305);
        assert_eq!(rv.state, State::Fetch);

        // ADD r1, r2, r4
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_out(),
            0b0000000_00001_00010_000_00100_0110011
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedInstruction {
                instruction: 0b0000000_00001_00010_000_00100_0110011,
                opcode: 0b0110011,
                rd: 0b00100,
                funct3: 0b000,
                imm11_0: 0b000000000001,
                rs1: 0x0203_0405,
                rs2: 0x0102_0304,
                funct7: 0b0000000,
                shamt: 0b00001
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                alu_result: 0x0305_0709,
                rd: 0b00100,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                alu_result: 0x0305_0709,
                rd: 0b00100,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b00100], 0x0305_0709);
        assert_eq!(rv.state, State::Fetch);

        // SUB r1, r2, r4
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_out(),
            0b0100000_00001_00010_000_00100_0110011
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedInstruction {
                instruction: 0b0100000_00001_00010_000_00100_0110011,
                opcode: 0b0110011,
                rd: 0b00100,
                funct3: 0b000,
                imm11_0: 0b010000000001,
                rs1: 0x0203_0405,
                rs2: 0x0102_0304,
                funct7: 0b0100000,
                shamt: 0b00001
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                alu_result: 0x0101_0101,
                rd: 0b00100,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                alu_result: 0x0101_0101,
                rd: 0b00100,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b00100], 0x0101_0101);
        assert_eq!(rv.state, State::Fetch);

        // ADDI 4095, r1, r3
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_out(),
            0b111111111111_00001_000_00011_0010011
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedInstruction {
                instruction: 0b111111111111_00001_000_00011_0010011,
                opcode: 0b0010011,
                rd: 0b00011,
                funct3: 0b000,
                imm11_0: 0b111111111111,
                rs1: 0x0102_0304,
                rs2: 0x0000_0000,
                funct7: 0b1111111,
                shamt: 0b11111
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                alu_result: 0x0102_1303,
                rd: 0b00011,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                alu_result: 0x0102_1303,
                rd: 0b00011,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b00011], 0x0102_1303);
        assert_eq!(rv.state, State::Fetch);

        // SRL r10, r11, r12
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_out(),
            0b0000000_01011_01010_101_01100_0110011
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedInstruction {
                instruction: 0b0000000_01011_01010_101_01100_0110011,
                opcode: 0b0110011,
                rd: 0b01100,
                funct3: 0b101,
                imm11_0: 0b000000001011,
                rs1: 0x8000_0000,
                rs2: 0x0000_0001,
                funct7: 0b0000000,
                shamt: 0b01011
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                alu_result: 0x4000_0000,
                rd: 0b01100,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                alu_result: 0x4000_0000,
                rd: 0b01100,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b01100], 0x4000_0000);
        assert_eq!(rv.state, State::Fetch);

        // SRA r10, r11, r12
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_out(),
            0b0100000_01011_01010_101_01100_0110011
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedInstruction {
                instruction: 0b0100000_01011_01010_101_01100_0110011,
                opcode: 0b0110011,
                rd: 0b01100,
                funct3: 0b101,
                imm11_0: 0b010000001011,
                rs1: 0x8000_0000,
                rs2: 0x0000_0001,
                funct7: 0b0100000,
                shamt: 0b01011
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                alu_result: 0xC000_0000,
                rd: 0b01100,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                alu_result: 0xC000_0000,
                rd: 0b01100,
                is_alu_operation: true,
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b01100], 0xC000_0000);
        assert_eq!(rv.state, State::Fetch);
    }
}
