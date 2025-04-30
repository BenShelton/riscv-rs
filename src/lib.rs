#![allow(dead_code)]
#![allow(clippy::unusual_byte_groupings)]

mod csr;
mod pipeline;
pub mod system_interface;
mod utils;

use csr::CSRInterface;
use pipeline::{
    PipelineStage,
    decode::{DecodedInstruction, InstructionDecode, InstructionDecodeParams},
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

pub struct RV32ISystem {
    pub bus: SystemInterface,
    pub csr: CSRInterface,
    pub state: State,
    pub reg_file: RegisterFile,
    stage_if: InstructionFetch,
    stage_de: InstructionDecode,
    stage_ex: InstructionExecute,
    stage_ma: InstructionMemoryAccess,
    stage_wb: InstructionWriteBack,
}

impl RV32ISystem {
    pub fn new() -> Self {
        let rom = RomDevice::new();
        let ram = RamDevice::new();

        Self {
            bus: SystemInterface::new(rom, ram),
            csr: CSRInterface::new(),
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
            branch_address: match self.stage_ex.get_execution_value_out().instruction {
                DecodedInstruction::Jal { branch_address, .. } => Some(branch_address),
                DecodedInstruction::Branch { branch_address, .. } => Some(branch_address),
                _ => None,
            },
            bus: &self.bus,
        });
        self.stage_de.compute(InstructionDecodeParams {
            should_stall: self.state != State::Decode,
            instruction_in: self.stage_if.get_instruction_value_out(),
            reg_file: &mut self.reg_file,
        });
        self.stage_ex.compute(InstructionExecuteParams {
            should_stall: self.state != State::Execute,
            decoded_instruction_in: self.stage_de.get_decoded_instruction_out(),
        });
        self.stage_ma.compute(InstructionMemoryAccessParams {
            should_stall: self.state != State::MemoryAccess,
            execution_value_in: self.stage_ex.get_execution_value_out(),
            bus: &mut self.bus,
            csr: &mut self.csr,
        });
        self.stage_wb.compute(InstructionWriteBackParams {
            should_stall: self.state != State::WriteBack,
            memory_access_value_in: self.stage_ma.get_memory_access_value_out(),
            reg_file: &mut self.reg_file,
        });
        self.csr.compute();
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
            State::WriteBack => {
                self.csr.instret.set(self.csr.instret.get() + 1);
                State::Fetch
            }
        };

        self.csr.latch_next();
    }

    pub fn current_line(&self) -> u32 {
        self.stage_if.get_instruction_value_out().pc
    }
}

impl Default for RV32ISystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pipeline::{
            decode::{DecodedInstruction, DecodedValue},
            execute::ExecutionValue,
            fetch::InstructionValue,
            memory_access::MemoryAccessValue,
        },
        system_interface::MMIODevice,
    };

    macro_rules! run_instruction {
        ($rv:expr) => {
            $rv.cycle();
            $rv.cycle();
            $rv.cycle();
            $rv.cycle();
            $rv.cycle();
            assert_eq!($rv.state, State::Fetch);
        };
    }

    #[test]
    fn test_rom_read() {
        let mut rv = RV32ISystem::new();
        rv.bus.rom.load(vec![0xDEAD_BEEF, 0xC0DE_CAFE]);
        assert_eq!(rv.bus.read_word(0x1000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.read_word(0x1000_0004), 0xC0DE_CAFE);
        assert_eq!(rv.bus.read_word(0x1000_0008), 0xFFFF_FFFF);
    }

    #[test]
    fn test_rom_write_does_nothing() {
        let mut rv = RV32ISystem::new();
        rv.bus.write_word(0x1000_0000, 0xDEAD_BEEF);
        rv.bus.write_word(0x1000_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.read_word(0x1000_0000), 0xFFFF_FFFF);
        assert_eq!(rv.bus.read_word(0x1000_0004), 0xFFFF_FFFF);
    }

    #[test]
    fn test_ram_write_read() {
        let mut rv = RV32ISystem::new();
        rv.bus.write_word(0x2000_0000, 0xDEAD_BEEF);
        rv.bus.write_word(0x2000_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.read_word(0x2000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.read_word(0x2000_0004), 0xC0DE_CAFE);
    }

    #[test]
    fn test_ram_write_wrap_around() {
        let mut rv = RV32ISystem::new();
        rv.bus.write_word(0x2040_0000, 0xDEAD_BEEF);
        rv.bus.write_word(0x2040_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.read_word(0x2000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.read_word(0x2000_0004), 0xC0DE_CAFE);
    }

    #[test]
    #[should_panic(expected = "Unaligned read from address 0x10000005")]
    fn test_panic_on_misaligned_read() {
        let rv = RV32ISystem::new();
        rv.bus.read_word(0x1000_0005);
    }

    #[test]
    #[should_panic(expected = "Unaligned write to address 0x10000005 (value=0xDEADBEEF)")]
    fn test_panic_on_misaligned_write() {
        let mut rv = RV32ISystem::new();
        rv.bus.write_word(0x1000_0005, 0xDEAD_BEEF);
    }

    #[test]
    fn test_alu_instructions() {
        let mut rv = RV32ISystem::new();
        rv.reg_file[1] = 0x0102_0304;
        rv.reg_file[2] = 0x0203_0405;
        rv.reg_file[10] = 0x8000_0000;
        rv.reg_file[11] = 0x0000_0001;

        rv.bus.rom.load(vec![
            0b000000000001_00001_000_00011_0010011,  // ADDI 1, r1, r3
            0b0000000_00001_00010_000_00100_0110011, // ADD r1, r2, r4
            0b0100000_00001_00010_000_00100_0110011, // SUB r1, r2, r4
            0b111111111111_00001_000_00011_0010011,  // ADDI -1, r1, r3
            0b0000000_01011_01010_101_01100_0110011, // SRL r10, r11, r12
            0b0100000_01011_01010_101_01100_0110011, // SRA r10, r11, r12
        ]);

        // ADDI 1, r1, r3
        let pc = 0x1000_0000;
        let pc_plus_4 = 0x1000_0004;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                instruction: 0b000000000001_00001_000_00011_0010011
            }
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00011,
                    funct3: 0b000,
                    imm11_0: 0b000000000001,
                    rs1: 0x0102_0304,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b000000000001,
                }
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                write_back_value: 0x0102_0305,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00011,
                    funct3: 0b000,
                    imm11_0: 0b000000000001,
                    rs1: 0x0102_0304,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b000000000001,
                }
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                write_back_value: 0x0102_0305,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00011,
                    funct3: 0b000,
                    imm11_0: 0b000000000001,
                    rs1: 0x0102_0304,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b000000000001,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b00011], 0x0102_0305);
        assert_eq!(rv.state, State::Fetch);

        // ADD r1, r2, r4
        let pc = 0x1000_0004;
        let pc_plus_4 = 0x1000_0008;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                instruction: 0b0000000_00001_00010_000_00100_0110011
            }
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b00100,
                    funct3: 0b000,
                    imm11_0: 0b000000000001,
                    rs1: 0x0203_0405,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b000000000001,
                }
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                write_back_value: 0x0305_0709,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b00100,
                    funct3: 0b000,
                    imm11_0: 0b000000000001,
                    rs1: 0x0203_0405,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b000000000001,
                }
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                write_back_value: 0x0305_0709,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b00100,
                    funct3: 0b000,
                    imm11_0: 0b000000000001,
                    rs1: 0x0203_0405,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b000000000001,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b00100], 0x0305_0709);
        assert_eq!(rv.state, State::Fetch);

        // SUB r1, r2, r4
        let pc = 0x1000_0008;
        let pc_plus_4 = 0x1000_000C;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                instruction: 0b0100000_00001_00010_000_00100_0110011
            }
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b00100,
                    funct3: 0b000,
                    imm11_0: 0b010000000001,
                    rs1: 0x0203_0405,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b010000000001,
                }
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                write_back_value: 0x0101_0101,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b00100,
                    funct3: 0b000,
                    imm11_0: 0b010000000001,
                    rs1: 0x0203_0405,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b010000000001,
                }
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                write_back_value: 0x0101_0101,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b00100,
                    funct3: 0b000,
                    imm11_0: 0b010000000001,
                    rs1: 0x0203_0405,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b010000000001,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b00100], 0x0101_0101);
        assert_eq!(rv.state, State::Fetch);

        // ADDI -1, r1, r3
        let pc = 0x1000_000C;
        let pc_plus_4 = 0x1000_0010;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                instruction: 0b111111111111_00001_000_00011_0010011
            }
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00011,
                    funct3: 0b000,
                    imm11_0: 0b111111111111,
                    rs1: 0x0102_0304,
                    rs2: 0x0000_0000,
                    shamt: 0b11111,
                    imm32: -1,
                }
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                write_back_value: 0x0102_0303,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00011,
                    funct3: 0b000,
                    imm11_0: 0b111111111111,
                    rs1: 0x0102_0304,
                    rs2: 0x0000_0000,
                    shamt: 0b11111,
                    imm32: -1,
                }
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                write_back_value: 0x0102_0303,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00011,
                    funct3: 0b000,
                    imm11_0: 0b111111111111,
                    rs1: 0x0102_0304,
                    rs2: 0x0000_0000,
                    shamt: 0b11111,
                    imm32: -1,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b00011], 0x0102_0303);
        assert_eq!(rv.state, State::Fetch);

        // SRL r10, r11, r12
        let pc = 0x1000_0010;
        let pc_plus_4 = 0x1000_0014;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                instruction: 0b0000000_01011_01010_101_01100_0110011
            }
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b01100,
                    funct3: 0b101,
                    imm11_0: 0b000000001011,
                    rs1: 0x8000_0000,
                    rs2: 0x0000_0001,
                    shamt: 0b01011,
                    imm32: 0b000000001011,
                }
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                write_back_value: 0x4000_0000,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b01100,
                    funct3: 0b101,
                    imm11_0: 0b000000001011,
                    rs1: 0x8000_0000,
                    rs2: 0x0000_0001,
                    shamt: 0b01011,
                    imm32: 0b000000001011,
                }
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                write_back_value: 0x4000_0000,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b01100,
                    funct3: 0b101,
                    imm11_0: 0b000000001011,
                    rs1: 0x8000_0000,
                    rs2: 0x0000_0001,
                    shamt: 0b01011,
                    imm32: 0b000000001011,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b01100], 0x4000_0000);
        assert_eq!(rv.state, State::Fetch);

        // SRA r10, r11, r12
        let pc = 0x1000_0014;
        let pc_plus_4 = 0x1000_0018;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                instruction: 0b0100000_01011_01010_101_01100_0110011
            }
        );
        assert_eq!(rv.state, State::Decode);

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b01100,
                    funct3: 0b101,
                    imm11_0: 0b010000001011,
                    rs1: 0x8000_0000,
                    rs2: 0x0000_0001,
                    shamt: 0b01011,
                    imm32: 0b010000001011,
                }
            }
        );
        assert_eq!(rv.state, State::Execute);

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                write_back_value: 0xC000_0000,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b01100,
                    funct3: 0b101,
                    imm11_0: 0b010000001011,
                    rs1: 0x8000_0000,
                    rs2: 0x0000_0001,
                    shamt: 0b01011,
                    imm32: 0b010000001011,
                }
            }
        );
        assert_eq!(rv.state, State::MemoryAccess);

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                write_back_value: 0xC000_0000,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b01100,
                    funct3: 0b101,
                    imm11_0: 0b010000001011,
                    rs1: 0x8000_0000,
                    rs2: 0x0000_0001,
                    shamt: 0b01011,
                    imm32: 0b010000001011,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);

        rv.cycle();
        assert_eq!(rv.reg_file[0b01100], 0xC000_0000);
        assert_eq!(rv.state, State::Fetch);
    }

    #[test]
    fn test_store_instructions() {
        let mut rv = RV32ISystem::new();
        // base
        rv.reg_file[1] = 0x2000_0000;
        // values to write
        rv.reg_file[2] = 0xDEAD_BEEF;
        rv.reg_file[3] = 0xC0DE_CAFE;
        rv.reg_file[4] = 0xABAD_1DEA;

        rv.bus.rom.load(vec![
            0b0000000_00010_00001_010_00100_0100011, // SW r2, r1, imm4
            0b0000000_00011_00001_001_00110_0100011, // SHW r3, r1, imm6
            0b0000000_00100_00001_000_00101_0100011, // SB r4, r1, imm5
        ]);

        // SW r2, r1, imm4
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc: 0x1000_0000,
                pc_plus_4: 0x1000_0004,
                write_back_value: 0x0000_0000,
                instruction: DecodedInstruction::Store {
                    funct3: 0b010,
                    rs1: 0x2000_0000,
                    rs2: 0xDEAD_BEEF,
                    imm32: 0b100,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);
        rv.cycle();
        assert_eq!(rv.bus.read_word(0x2000_0004), 0xDEAD_BEEF);
        assert_eq!(rv.state, State::Fetch);

        // SHW r3, r1, imm6
        run_instruction!(rv);
        assert_eq!(rv.bus.read_word(0x2000_0004), 0xDEAD_CAFE);

        // SB r4, r1, imm5
        run_instruction!(rv);
        assert_eq!(rv.bus.read_word(0x2000_0004), 0xDEEA_CAFE);
        assert_eq!(rv.bus.read_half_word(0x2000_0004), 0xDEEA);
        assert_eq!(rv.bus.read_half_word(0x2000_0006), 0xCAFE);
        assert_eq!(rv.bus.read_byte(0x2000_0004), 0xDE);
        assert_eq!(rv.bus.read_byte(0x2000_0005), 0xEA);
        assert_eq!(rv.bus.read_byte(0x2000_0006), 0xCA);
        assert_eq!(rv.bus.read_byte(0x2000_0007), 0xFE);

        // start with fresh state
        let mut rv = RV32ISystem::new();
        rv.reg_file[1] = 0x2000_0005;
        rv.reg_file[2] = 0xDEAD_BEEF;

        rv.bus.rom.load(vec![
            0b1111111_00010_00001_010_11111_0100011, // SW r2, r1, imm-1
        ]);

        // SW r2, r1, imm-1
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc: 0x1000_0000,
                pc_plus_4: 0x1000_0004,
                write_back_value: 0x0000_0000,
                instruction: DecodedInstruction::Store {
                    funct3: 0b010,
                    rs1: 0x2000_0005,
                    rs2: 0xDEAD_BEEF,
                    imm32: -1,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);
        rv.cycle();
        assert_eq!(rv.bus.read_word(0x2000_0004), 0xDEAD_BEEF);
        assert_eq!(rv.state, State::Fetch);
    }

    #[test]
    fn test_load_instructions() {
        let mut rv = RV32ISystem::new();
        rv.reg_file[1] = 0x2000_0000;
        rv.reg_file[10] = 0x2000_0005;
        rv.bus.write_word(0x2000_0004, 0xDEADBEEF);

        rv.bus.rom.load(vec![
            0b000000000100_00001_010_00010_0000011, // LW r2, r1, imm4
            0b000000000110_00001_001_00011_0000011, // LHW r3, r1, imm6
            0b000000000111_00001_000_00100_0000011, // LB r4, r1, imm7
            0b000000000110_00001_101_00101_0000011, // LHWU r5, r1, imm6
            0b000000000111_00001_100_00110_0000011, // LBU r6, r1, imm7
            0b111111111111_01010_010_01011_0000011, // LW r11, r10, imm-1
        ]);

        // LW r2, r1, imm4
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc: 0x1000_0000,
                pc_plus_4: 0x1000_0004,
                write_back_value: 0xDEAD_BEEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b010,
                    rs1: 0x2000_0000,
                    rd: 0b00010,
                    imm32: 0b100,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);
        rv.cycle();
        assert_eq!(rv.reg_file[2], 0xDEAD_BEEF);
        assert_eq!(rv.state, State::Fetch);

        // LHW r3, r1, imm6
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc: 0x1000_0004,
                pc_plus_4: 0x1000_0008,
                write_back_value: 0xFFFF_BEEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b001,
                    rs1: 0x2000_0000,
                    rd: 0b00011,
                    imm32: 0b110,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);
        rv.cycle();
        assert_eq!(rv.reg_file[3], 0xFFFF_BEEF);
        assert_eq!(rv.state, State::Fetch);

        // LB r4, r1, imm7
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc: 0x1000_0008,
                pc_plus_4: 0x1000_000C,
                write_back_value: 0xFFFF_FFEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b000,
                    rs1: 0x2000_0000,
                    rd: 0b00100,
                    imm32: 0b111,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);
        rv.cycle();
        assert_eq!(rv.reg_file[4], 0xFFFF_FFEF);
        assert_eq!(rv.state, State::Fetch);

        // LHWU r5, r1, imm6
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc: 0x1000_000C,
                pc_plus_4: 0x1000_0010,
                write_back_value: 0x0000_BEEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b101,
                    rs1: 0x2000_0000,
                    rd: 0b00101,
                    imm32: 0b110,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);
        rv.cycle();
        assert_eq!(rv.reg_file[5], 0x0000_BEEF);
        assert_eq!(rv.state, State::Fetch);

        // LBU r6, r1, imm7
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc: 0x1000_0010,
                pc_plus_4: 0x1000_0014,
                write_back_value: 0x0000_00EF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b100,
                    rs1: 0x2000_0000,
                    rd: 0b00110,
                    imm32: 0b111,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);
        rv.cycle();
        assert_eq!(rv.reg_file[6], 0x0000_00EF);
        assert_eq!(rv.state, State::Fetch);

        // LW r11, r10, imm-1
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc: 0x1000_0014,
                pc_plus_4: 0x1000_0018,
                write_back_value: 0xDEAD_BEEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b010,
                    rs1: 0x2000_0005,
                    rd: 0b01011,
                    imm32: -1,
                }
            }
        );
        assert_eq!(rv.state, State::WriteBack);
        rv.cycle();
        assert_eq!(rv.reg_file[11], 0xDEAD_BEEF);
        assert_eq!(rv.state, State::Fetch);
    }

    #[test]
    fn test_lui_instructions() {
        let mut rv = RV32ISystem::new();

        rv.bus.rom.load(vec![
            0b10101010101010101010_00001_0110111,   // LUI r1, 0xAAAAA
            0b101010101010_00001_000_00001_0010011, // ADDI r1, r1, 0xAAA
        ]);

        // LUI r1, 0xAAAAA
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc: 0x1000_0000,
                pc_plus_4: 0x1000_0004,
                instruction: DecodedInstruction::Lui {
                    rd: 0b00001,
                    imm32: 0b10101010101010101010_000000000000,
                }
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(rv.state, State::Fetch);
        assert_eq!(rv.reg_file[1], 0xAAAA_A000);

        // ADDI r1, 0xAAAAA
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc: 0x1000_0004,
                pc_plus_4: 0x1000_0008,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00001,
                    funct3: 0b000,
                    imm11_0: 0xAAA,
                    rs1: 0xAAAA_A000,
                    rs2: 0b000,
                    shamt: 0b01010,
                    imm32: -1366,
                }
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(rv.state, State::Fetch);
        assert_eq!(rv.reg_file[1], 0xAAAA_9AAA);
    }

    #[test]
    fn test_jal_instructions() {
        let mut rv = RV32ISystem::new();

        rv.bus.rom.load(vec![
            0,
            0,
            0b0_0000011110_0_00000000_00000_1101111, // JAL r0, 0x44
            0,                                       // second jump lands here
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0b000000000000_00001_000_00000_1100111, // JALR x0, 0
            0,                                      // first jump lands here
            0,
            0,
            0,
            0b1_1111011100_1_11111111_00001_1101111, // JAL r1, 0xFFFDC
            0,                                       // third jump returns here
        ]);

        for _ in 0..2 {
            run_instruction!(rv);
        }

        // JAL r0, 0x44
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc: 0x1000_0008,
                pc_plus_4: 0x1000_000C,
                instruction: DecodedInstruction::Jal {
                    rd: 0b00000,
                    branch_address: 0x1000_0044,
                }
            }
        );
        assert_eq!(rv.state, State::Execute);
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(rv.state, State::Fetch);

        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc: 0x1000_0044,
                pc_plus_4: 0x1000_0048,
                instruction: 0,
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(rv.state, State::Fetch);

        for _ in 0..3 {
            run_instruction!(rv);
        }

        // JAL r1, 0xFFFDC
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                instruction: 0b1_1111011100_1_11111111_00001_1101111,
                pc: 0x1000_0054,
                pc_plus_4: 0x1000_0058,
            }
        );
        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc: 0x1000_0054,
                pc_plus_4: 0x1000_0058,
                instruction: DecodedInstruction::Jal {
                    rd: 0b00001,
                    branch_address: 0x1000_000C,
                }
            }
        );
        assert_eq!(rv.state, State::Execute);
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(rv.state, State::Fetch);

        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc: 0x1000_000C,
                pc_plus_4: 0x1000_0010,
                instruction: 0,
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(rv.state, State::Fetch);

        for _ in 0..12 {
            run_instruction!(rv);
        }

        // JALR x0, 0
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                instruction: 0b000000000000_00001_000_00000_1100111,
                pc: 0x1000_0040,
                pc_plus_4: 0x1000_0044,
            }
        );
        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc: 0x1000_0040,
                pc_plus_4: 0x1000_0044,
                instruction: DecodedInstruction::Jal {
                    rd: 0b00000,
                    branch_address: 0x1000_0058,
                }
            }
        );
        assert_eq!(rv.state, State::Execute);
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(rv.state, State::Fetch);

        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc: 0x1000_0058,
                pc_plus_4: 0x1000_005C,
                instruction: 0,
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(rv.state, State::Fetch);
    }
}
