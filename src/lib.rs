#![allow(dead_code)]
#![allow(clippy::unusual_byte_groupings)]

mod csr;
mod pipeline;
pub mod system_interface;
pub mod trap;
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
use trap::{TrapInterface, TrapParams};
use utils::LatchValue;

use crate::pipeline::{decode::DecodedValue, memory_access::MemoryAccessValue};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum CPUState {
    Pipeline(PipelineState),
    Trap,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum PipelineState {
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
    pub trap: TrapInterface,
    pub state: LatchValue<CPUState>,
    pub reg_file: RegisterFile,
    /// This is acting as a combinational signal, not a reg
    pub trap_stall: bool,
    /// This is acting as a combinational signal, not a reg
    pub mret: bool,
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
            trap: TrapInterface::new(),
            state: LatchValue::new(CPUState::Pipeline(PipelineState::Fetch)),
            reg_file: [0u32; 32],
            trap_stall: false,
            mret: false,
            stage_if: InstructionFetch::new(),
            stage_de: InstructionDecode::new(),
            stage_ex: InstructionExecute::new(),
            stage_ma: InstructionMemoryAccess::new(),
            stage_wb: InstructionWriteBack::new(),
        }
    }

    pub fn compute(&mut self) {
        let dec_values = self.stage_de.get_decoded_instruction_out();
        let mem_values = self.stage_ma.get_memory_access_value_out();

        self.mret = dec_values.return_from_trap;

        // prefer traps later in the pipeline
        let trap_params = match (dec_values, mem_values) {
            (DecodedValue { .. }, MemoryAccessValue { trap_params, .. }) if trap_params.trap => {
                Some(trap_params)
            }
            (DecodedValue { trap_params, .. }, MemoryAccessValue { .. }) if trap_params.trap => {
                Some(trap_params)
            }
            _ => None,
        };
        self.trap_stall = self.state.get() == &CPUState::Trap || trap_params.is_some() || self.mret;

        if self.trap_stall && matches!(self.state.get(), &CPUState::Pipeline(_)) {
            self.state.set(CPUState::Trap);

            if let Some(trap_params) = trap_params {
                self.trap.mcause.set(trap_params.mcause);
                self.trap.mepc.set(trap_params.mepc);
                self.trap.mtval.set(trap_params.mtval);
            }
        } else if self.state.get() == &CPUState::Trap && *self.trap.return_to_pipeline_mode.get() {
            self.state.set(CPUState::Pipeline(PipelineState::Fetch));
            if *self.trap.set_pc.get() {
                self.stage_if.pc.set(*self.trap.pc_to_set.get());
                self.stage_if.pc_plus_4.set(*self.trap.pc_to_set.get());
            }
        }

        if matches!(self.state.get(), &CPUState::Pipeline(_)) && self.mret {
            self.state.set(CPUState::Trap);
        }

        if *self.trap.flush.get() {
            self.stage_if.reset();
            self.stage_de.reset();
            self.stage_ex.reset();
            self.stage_ma.reset();
            self.stage_wb.reset();
        }

        self.stage_if.compute(InstructionFetchParams {
            should_stall: self.trap_stall
                || *self.state.get() != CPUState::Pipeline(PipelineState::Fetch),
            branch_address: match self.stage_ex.get_execution_value_out().instruction {
                DecodedInstruction::Jal { branch_address, .. } => Some(branch_address),
                DecodedInstruction::Branch { branch_address, .. } => Some(branch_address),
                _ => None,
            },
            bus: &self.bus,
        });
        self.stage_de.compute(InstructionDecodeParams {
            should_stall: self.trap_stall
                || *self.state.get() != CPUState::Pipeline(PipelineState::Decode),
            instruction_in: self.stage_if.get_instruction_value_out(),
            reg_file: &mut self.reg_file,
        });
        self.stage_ex.compute(InstructionExecuteParams {
            should_stall: self.trap_stall
                || *self.state.get() != CPUState::Pipeline(PipelineState::Execute),
            decoded_instruction_in: self.stage_de.get_decoded_instruction_out(),
        });
        self.stage_ma.compute(InstructionMemoryAccessParams {
            should_stall: self.trap_stall
                || *self.state.get() != CPUState::Pipeline(PipelineState::MemoryAccess),
            execution_value_in: self.stage_ex.get_execution_value_out(),
            bus: &mut self.bus,
            csr: &mut self.csr,
        });
        self.stage_wb.compute(InstructionWriteBackParams {
            should_stall: self.trap_stall
                || *self.state.get() != CPUState::Pipeline(PipelineState::WriteBack),
            memory_access_value_in: self.stage_ma.get_memory_access_value_out(),
            reg_file: &mut self.reg_file,
        });
        self.csr.compute();
        self.trap.compute(TrapParams {
            csr: &mut self.csr,
            begin_trap: self.stage_de.get_decoded_instruction_out().trap_params.trap
                || self.stage_ma.get_memory_access_value_out().trap_params.trap,
            begin_trap_return: self.stage_de.get_decoded_instruction_out().return_from_trap,
        });

        if !self.trap_stall {
            self.state.set(match *self.state.get() {
                CPUState::Pipeline(PipelineState::Fetch) => {
                    CPUState::Pipeline(PipelineState::Decode)
                }
                CPUState::Pipeline(PipelineState::Decode) => {
                    CPUState::Pipeline(PipelineState::Execute)
                }
                CPUState::Pipeline(PipelineState::Execute) => {
                    CPUState::Pipeline(PipelineState::MemoryAccess)
                }
                CPUState::Pipeline(PipelineState::MemoryAccess) => {
                    CPUState::Pipeline(PipelineState::WriteBack)
                }
                CPUState::Pipeline(PipelineState::WriteBack) => {
                    self.csr.instret.set(self.csr.instret.get() + 1);
                    CPUState::Pipeline(PipelineState::Fetch)
                }
                _ => *self.state.get(),
            });
        }
    }

    pub fn latch_next(&mut self) {
        self.stage_if.latch_next();
        self.stage_de.latch_next();
        self.stage_ex.latch_next();
        self.stage_ma.latch_next();
        self.stage_wb.latch_next();
        self.csr.latch_next();
        self.trap.latch_next();
        self.state.latch_next();
    }

    pub fn cycle(&mut self) {
        self.compute();
        self.latch_next();
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
        trap::{MCAUSE_LOAD_ADDRESS_MISALIGNED, PipelineTrapParams, TrapState},
    };

    macro_rules! run_instruction {
        ($rv:expr) => {
            $rv.cycle();
            $rv.cycle();
            $rv.cycle();
            $rv.cycle();
            $rv.cycle();
            assert_eq!(*$rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));
        };
    }

    #[test]
    fn test_rom_read() {
        let mut rv = RV32ISystem::new();
        rv.bus.rom.load(vec![0xDEAD_BEEF, 0xC0DE_CAFE]);
        assert_eq!(rv.bus.read_word(0x1000_0000), Ok(0xDEAD_BEEF));
        assert_eq!(rv.bus.read_word(0x1000_0004), Ok(0xC0DE_CAFE));
        assert_eq!(rv.bus.read_word(0x1000_0008), Ok(0xFFFF_FFFF));
    }

    #[test]
    fn test_rom_write_does_nothing() {
        let mut rv = RV32ISystem::new();
        rv.bus.write_word(0x1000_0000, 0xDEAD_BEEF).unwrap();
        rv.bus.write_word(0x1000_0004, 0xC0DE_CAFE).unwrap();
        assert_eq!(rv.bus.read_word(0x1000_0000), Ok(0xFFFF_FFFF));
        assert_eq!(rv.bus.read_word(0x1000_0004), Ok(0xFFFF_FFFF));
    }

    #[test]
    fn test_ram_write_read() {
        let mut rv = RV32ISystem::new();
        rv.bus.write_word(0x2000_0000, 0xDEAD_BEEF).unwrap();
        rv.bus.write_word(0x2000_0004, 0xC0DE_CAFE).unwrap();
        assert_eq!(rv.bus.read_word(0x2000_0000), Ok(0xDEAD_BEEF));
        assert_eq!(rv.bus.read_word(0x2000_0004), Ok(0xC0DE_CAFE));
    }

    #[test]
    fn test_ram_write_wrap_around() {
        let mut rv = RV32ISystem::new();
        rv.bus.write_word(0x2040_0000, 0xDEAD_BEEF).unwrap();
        rv.bus.write_word(0x2040_0004, 0xC0DE_CAFE).unwrap();
        assert_eq!(rv.bus.read_word(0x2000_0000), Ok(0xDEAD_BEEF));
        assert_eq!(rv.bus.read_word(0x2000_0004), Ok(0xC0DE_CAFE));
    }

    #[test]
    fn test_error_on_misaligned_read() {
        let rv = RV32ISystem::new();
        assert_eq!(
            rv.bus.read_word(0x1000_0005).map_err(|e| format!("{}", e)),
            Err("Unaligned read from address 0x10000005".to_string())
        );
    }

    #[test]
    fn test_error_on_misaligned_write() {
        let mut rv = RV32ISystem::new();
        assert_eq!(
            rv.bus
                .write_word(0x1000_0005, 0xDEAD_BEEF)
                .map_err(|e| format!("{}", e)),
            Err("Unaligned write to address 0x10000005 (value=0xDEADBEEF)".to_string())
        );
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
        let raw_instruction = 0b000000000001_00001_000_00011_0010011;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                raw_instruction,
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Decode));

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                raw_instruction,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00011,
                    funct3: 0b000,
                    imm11_0: 0b000000000001,
                    rs1: 0x0102_0304,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b000000000001,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::MemoryAccess)
        );

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );

        rv.cycle();
        assert_eq!(rv.reg_file[0b00011], 0x0102_0305);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        // ADD r1, r2, r4
        let pc = 0x1000_0004;
        let pc_plus_4 = 0x1000_0008;
        let raw_instruction = 0b0000000_00001_00010_000_00100_0110011;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                raw_instruction,
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Decode));

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                raw_instruction,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b00100,
                    funct3: 0b000,
                    imm11_0: 0b000000000001,
                    rs1: 0x0203_0405,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b000000000001,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::MemoryAccess)
        );

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );

        rv.cycle();
        assert_eq!(rv.reg_file[0b00100], 0x0305_0709);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        // SUB r1, r2, r4
        let pc = 0x1000_0008;
        let pc_plus_4 = 0x1000_000C;
        let raw_instruction = 0b0100000_00001_00010_000_00100_0110011;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                raw_instruction,
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Decode));

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                raw_instruction,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b00100,
                    funct3: 0b000,
                    imm11_0: 0b010000000001,
                    rs1: 0x0203_0405,
                    rs2: 0x0102_0304,
                    shamt: 0b00001,
                    imm32: 0b010000000001,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::MemoryAccess)
        );

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );

        rv.cycle();
        assert_eq!(rv.reg_file[0b00100], 0x0101_0101);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        // ADDI -1, r1, r3
        let pc = 0x1000_000C;
        let pc_plus_4 = 0x1000_0010;
        let raw_instruction = 0b111111111111_00001_000_00011_0010011;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                raw_instruction,
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Decode));

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                raw_instruction,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00011,
                    funct3: 0b000,
                    imm11_0: 0b111111111111,
                    rs1: 0x0102_0304,
                    rs2: 0x0000_0000,
                    shamt: 0b11111,
                    imm32: -1,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::MemoryAccess)
        );

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );

        rv.cycle();
        assert_eq!(rv.reg_file[0b00011], 0x0102_0303);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        // SRL r10, r11, r12
        let pc = 0x1000_0010;
        let pc_plus_4 = 0x1000_0014;
        let raw_instruction = 0b0000000_01011_01010_101_01100_0110011;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                raw_instruction,
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Decode));

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                raw_instruction,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b01100,
                    funct3: 0b101,
                    imm11_0: 0b000000001011,
                    rs1: 0x8000_0000,
                    rs2: 0x0000_0001,
                    shamt: 0b01011,
                    imm32: 0b000000001011,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::MemoryAccess)
        );

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );

        rv.cycle();
        assert_eq!(rv.reg_file[0b01100], 0x4000_0000);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        // SRA r10, r11, r12
        let pc = 0x1000_0014;
        let pc_plus_4 = 0x1000_0018;
        let raw_instruction = 0b0100000_01011_01010_101_01100_0110011;
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc,
                pc_plus_4,
                raw_instruction,
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Decode));

        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc,
                pc_plus_4,
                raw_instruction,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0110011,
                    rd: 0b01100,
                    funct3: 0b101,
                    imm11_0: 0b010000001011,
                    rs1: 0x8000_0000,
                    rs2: 0x0000_0001,
                    shamt: 0b01011,
                    imm32: 0b010000001011,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));

        rv.cycle();
        assert_eq!(
            rv.stage_ex.get_execution_value_out(),
            ExecutionValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::MemoryAccess)
        );

        rv.cycle();
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc,
                pc_plus_4,
                raw_instruction,
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
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );

        rv.cycle();
        assert_eq!(rv.reg_file[0b01100], 0xC000_0000);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));
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
                raw_instruction: 0b0000000_00010_00001_010_00100_0100011,
                write_back_value: 0x0000_0000,
                instruction: DecodedInstruction::Store {
                    funct3: 0b010,
                    rs1: 0x2000_0000,
                    rs2: 0xDEAD_BEEF,
                    imm32: 0b100,
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );
        rv.cycle();
        assert_eq!(rv.bus.read_word(0x2000_0004), Ok(0xDEAD_BEEF));
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        // SHW r3, r1, imm6
        run_instruction!(rv);
        assert_eq!(rv.bus.read_word(0x2000_0004), Ok(0xDEAD_CAFE));

        // SB r4, r1, imm5
        run_instruction!(rv);
        assert_eq!(rv.bus.read_word(0x2000_0004), Ok(0xDEEA_CAFE));
        assert_eq!(rv.bus.read_half_word(0x2000_0004), Ok(0xDEEA));
        assert_eq!(rv.bus.read_half_word(0x2000_0006), Ok(0xCAFE));
        assert_eq!(rv.bus.read_byte(0x2000_0004), Ok(0xDE));
        assert_eq!(rv.bus.read_byte(0x2000_0005), Ok(0xEA));
        assert_eq!(rv.bus.read_byte(0x2000_0006), Ok(0xCA));
        assert_eq!(rv.bus.read_byte(0x2000_0007), Ok(0xFE));

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
                raw_instruction: 0b1111111_00010_00001_010_11111_0100011,
                write_back_value: 0x0000_0000,
                instruction: DecodedInstruction::Store {
                    funct3: 0b010,
                    rs1: 0x2000_0005,
                    rs2: 0xDEAD_BEEF,
                    imm32: -1,
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );
        rv.cycle();
        assert_eq!(rv.bus.read_word(0x2000_0004), Ok(0xDEAD_BEEF));
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));
    }

    #[test]
    fn test_load_instructions() {
        let mut rv = RV32ISystem::new();
        rv.reg_file[1] = 0x2000_0000;
        rv.reg_file[10] = 0x2000_0005;
        rv.bus.write_word(0x2000_0004, 0xDEADBEEF).unwrap();

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
                raw_instruction: 0b000000000100_00001_010_00010_0000011,
                write_back_value: 0xDEAD_BEEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b010,
                    rs1: 0x2000_0000,
                    rd: 0b00010,
                    imm32: 0b100,
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );
        rv.cycle();
        assert_eq!(rv.reg_file[2], 0xDEAD_BEEF);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

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
                raw_instruction: 0b000000000110_00001_001_00011_0000011,
                write_back_value: 0xFFFF_BEEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b001,
                    rs1: 0x2000_0000,
                    rd: 0b00011,
                    imm32: 0b110,
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );
        rv.cycle();
        assert_eq!(rv.reg_file[3], 0xFFFF_BEEF);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

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
                raw_instruction: 0b000000000111_00001_000_00100_0000011,
                write_back_value: 0xFFFF_FFEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b000,
                    rs1: 0x2000_0000,
                    rd: 0b00100,
                    imm32: 0b111,
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );
        rv.cycle();
        assert_eq!(rv.reg_file[4], 0xFFFF_FFEF);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

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
                raw_instruction: 0b000000000110_00001_101_00101_0000011,
                write_back_value: 0x0000_BEEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b101,
                    rs1: 0x2000_0000,
                    rd: 0b00101,
                    imm32: 0b110,
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );
        rv.cycle();
        assert_eq!(rv.reg_file[5], 0x0000_BEEF);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

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
                raw_instruction: 0b000000000111_00001_100_00110_0000011,
                write_back_value: 0x0000_00EF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b100,
                    rs1: 0x2000_0000,
                    rd: 0b00110,
                    imm32: 0b111,
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );
        rv.cycle();
        assert_eq!(rv.reg_file[6], 0x0000_00EF);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

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
                raw_instruction: 0b111111111111_01010_010_01011_0000011,
                write_back_value: 0xDEAD_BEEF,
                instruction: DecodedInstruction::Load {
                    funct3: 0b010,
                    rs1: 0x2000_0005,
                    rd: 0b01011,
                    imm32: -1,
                },
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );
        rv.cycle();
        assert_eq!(rv.reg_file[11], 0xDEAD_BEEF);
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));
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
                raw_instruction: 0b10101010101010101010_00001_0110111,
                instruction: DecodedInstruction::Lui {
                    rd: 0b00001,
                    imm32: 0b10101010101010101010_000000000000,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));
        assert_eq!(rv.reg_file[1], 0xAAAA_A000);

        // ADDI r1, 0xAAAAA
        rv.cycle();
        rv.cycle();
        assert_eq!(
            rv.stage_de.get_decoded_instruction_out(),
            DecodedValue {
                pc: 0x1000_0004,
                pc_plus_4: 0x1000_0008,
                raw_instruction: 0b101010101010_00001_000_00001_0010011,
                instruction: DecodedInstruction::Alu {
                    opcode: 0b0010011,
                    rd: 0b00001,
                    funct3: 0b000,
                    imm11_0: 0xAAA,
                    rs1: 0xAAAA_A000,
                    rs2: 0b000,
                    shamt: 0b01010,
                    imm32: -1366,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));
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
                raw_instruction: 0b0_0000011110_0_00000000_00000_1101111,
                instruction: DecodedInstruction::Jal {
                    rd: 0b00000,
                    branch_address: 0x1000_0044,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc: 0x1000_0044,
                pc_plus_4: 0x1000_0048,
                raw_instruction: 0,
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        for _ in 0..3 {
            run_instruction!(rv);
        }

        // JAL r1, 0xFFFDC
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                raw_instruction: 0b1_1111011100_1_11111111_00001_1101111,
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
                raw_instruction: 0b1_1111011100_1_11111111_00001_1101111,
                instruction: DecodedInstruction::Jal {
                    rd: 0b00001,
                    branch_address: 0x1000_000C,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc: 0x1000_000C,
                pc_plus_4: 0x1000_0010,
                raw_instruction: 0,
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        for _ in 0..12 {
            run_instruction!(rv);
        }

        // JALR x0, 0
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                raw_instruction: 0b000000000000_00001_000_00000_1100111,
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
                raw_instruction: 0b000000000000_00001_000_00000_1100111,
                instruction: DecodedInstruction::Jal {
                    rd: 0b00000,
                    branch_address: 0x1000_0058,
                },
                return_from_trap: false,
                trap_params: PipelineTrapParams::default(),
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                pc: 0x1000_0058,
                pc_plus_4: 0x1000_005C,
                raw_instruction: 0,
            }
        );
        rv.cycle();
        rv.cycle();
        rv.cycle();
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));
    }

    #[test]
    fn test_memory_access_trap() {
        let mut rv = RV32ISystem::new();
        rv.reg_file[2] = 0x2000_0000;

        rv.bus.rom.load(vec![
            0b000000000001_00010_010_01110_0000011, // LW r14, r2, imm1
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
            0,
            0,
            0,
            0,
            0b000000000000_00001_000_00000_1100111, // JALR x0, 0 (fake exception jump)
        ]);

        // LW r14, r2, imm1
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Decode));
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Execute));
        rv.cycle();
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::MemoryAccess)
        );
        rv.cycle();
        assert_eq!(
            *rv.state.get(),
            CPUState::Pipeline(PipelineState::WriteBack)
        );
        assert_eq!(
            rv.stage_ma.get_memory_access_value_out(),
            MemoryAccessValue {
                pc: 0x1000_0000,
                pc_plus_4: 0x1000_0004,
                raw_instruction: 0b000000000001_00010_010_01110_0000011,
                write_back_value: 0x0000_0000,
                instruction: DecodedInstruction::Load {
                    funct3: 0b010,
                    rs1: 0x2000_0000,
                    rd: 0b01110,
                    imm32: 0b1,
                },
                trap_params: PipelineTrapParams {
                    mcause: MCAUSE_LOAD_ADDRESS_MISALIGNED,
                    mepc: 0x1000_0004,
                    mtval: 0b000000000001_00010_010_01110_0000011,
                    trap: true,
                },
            }
        );
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Trap);
        assert_eq!(*rv.trap.state.get(), TrapState::SetCSRJump);
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Trap);
        assert_eq!(*rv.trap.state.get(), TrapState::Idle);
        rv.cycle();
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Fetch));

        // JAL r0, 0x44 (fake exception jump)
        rv.cycle();
        assert_eq!(
            rv.stage_if.get_instruction_value_out(),
            InstructionValue {
                raw_instruction: 0b000000000000_00001_000_00000_1100111,
                pc: 0x1000_0044,
                pc_plus_4: 0x1000_0048,
            }
        );
        assert_eq!(*rv.state.get(), CPUState::Pipeline(PipelineState::Decode));
    }
}
