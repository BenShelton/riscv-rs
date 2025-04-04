#![allow(dead_code)]
#![allow(clippy::unusual_byte_groupings)]

mod pipeline;
mod system_interface;

use pipeline::{
    PipelineStage,
    decode::{InstructionDecode, InstructionDecodeParams},
    execute::{InstructionExecute, InstructionExecuteParams},
    fetch::{InstructionFetch, InstructionFetchParams},
};
use std::{cell::RefCell, rc::Rc};
use system_interface::{RamDevice, RomDevice, SystemInterface};

#[derive(PartialEq, Eq, Debug)]
enum State {
    Fetch,
    Decode,
    Execute,
    MemoryAccess,
    Writeback,
}

pub type RegisterFile = Rc<RefCell<[i32; 32]>>;

struct RVI32System {
    bus: Rc<RefCell<SystemInterface>>,
    state: Rc<RefCell<State>>,
    reg_file: RegisterFile,
    stage_if: Rc<RefCell<InstructionFetch>>,
    stage_de: Rc<RefCell<InstructionDecode>>,
    stage_ex: Rc<RefCell<InstructionExecute>>,
}

impl RVI32System {
    pub fn new() -> Self {
        let rom = RomDevice::new();
        let ram = RamDevice::new();
        let bus = Rc::new(RefCell::new(SystemInterface::new(rom, ram)));
        let reg_file = Rc::new(RefCell::new([0i32; 32]));

        let state = Rc::new(RefCell::new(State::Fetch));

        let stage_if = {
            let state = Rc::clone(&state);
            Rc::new(RefCell::new(InstructionFetch::new(
                InstructionFetchParams {
                    bus: Rc::clone(&bus),
                    should_stall: Box::new(move || *state.borrow() != State::Fetch),
                },
            )))
        };

        let stage_de = {
            let state = Rc::clone(&state);
            let stage_if = Rc::clone(&stage_if);
            Rc::new(RefCell::new(InstructionDecode::new(
                InstructionDecodeParams {
                    should_stall: Box::new(move || *state.borrow() != State::Decode),
                    get_instruction_in: Box::new(move || stage_if.borrow().get_instruction_out()),
                    reg_file: Rc::clone(&reg_file),
                },
            )))
        };

        let stage_ex = {
            let state = Rc::clone(&state);
            let stage_de = Rc::clone(&stage_de);
            Rc::new(RefCell::new(InstructionExecute::new(
                InstructionExecuteParams {
                    should_stall: Box::new(move || *state.borrow() != State::Execute),
                    get_decoded_instruction_in: Box::new(move || {
                        stage_de.borrow().get_decoded_instruction_out()
                    }),
                },
            )))
        };

        Self {
            bus: Rc::clone(&bus),
            state: Rc::clone(&state),
            reg_file: Rc::clone(&reg_file),
            stage_if: Rc::clone(&stage_if),
            stage_de: Rc::clone(&stage_de),
            stage_ex: Rc::clone(&stage_ex),
        }
    }

    pub fn compute(&mut self) {
        self.stage_if.borrow_mut().compute();
        self.stage_de.borrow_mut().compute();
        self.stage_ex.borrow_mut().compute();
    }

    pub fn latch_next(&mut self) {
        self.stage_if.borrow_mut().latch_next();
        self.stage_de.borrow_mut().latch_next();
        self.stage_ex.borrow_mut().latch_next();
    }

    pub fn cycle(&mut self) {
        self.compute();
        self.latch_next();

        self.state.replace_with(|state| match state {
            State::Fetch => State::Decode,
            State::Decode => State::Execute,
            _ => State::Fetch,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_interface::MMIODevice;

    #[test]
    fn test_rom_read() {
        let rv = RVI32System::new();
        rv.bus.borrow_mut().rom.load(vec![0xDEAD_BEEF, 0xC0DE_CAFE]);
        assert_eq!(rv.bus.borrow().read(0x1000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.borrow().read(0x1000_0004), 0xC0DE_CAFE);
        assert_eq!(rv.bus.borrow().read(0x1000_0008), 0xFFFF_FFFF);
    }

    #[test]
    fn test_rom_write_does_nothing() {
        let rv = RVI32System::new();
        rv.bus.borrow_mut().write(0x1000_0000, 0xDEAD_BEEF);
        rv.bus.borrow_mut().write(0x1000_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.borrow().read(0x1000_0000), 0xFFFF_FFFF);
        assert_eq!(rv.bus.borrow().read(0x1000_0004), 0xFFFF_FFFF);
    }

    #[test]
    fn test_ram_write_read() {
        let rv = RVI32System::new();
        rv.bus.borrow_mut().write(0x2000_0000, 0xDEAD_BEEF);
        rv.bus.borrow_mut().write(0x2000_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.borrow().read(0x2000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.borrow().read(0x2000_0004), 0xC0DE_CAFE);
    }

    #[test]
    fn test_ram_write_wrap_around() {
        let rv = RVI32System::new();
        rv.bus.borrow_mut().write(0x2040_0000, 0xDEAD_BEEF);
        rv.bus.borrow_mut().write(0x2040_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.borrow().read(0x2000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.borrow().read(0x2000_0004), 0xC0DE_CAFE);
    }

    #[test]
    #[should_panic(expected = "Unaligned read from address 0x10000005")]
    fn test_panic_on_misaligned_read() {
        let rv = RVI32System::new();
        rv.bus.borrow().read(0x1000_0005);
    }

    #[test]
    #[should_panic(expected = "Unaligned write to address 0x10000005 (value=0xDEADBEEF)")]
    fn test_panic_on_misaligned_write() {
        let rv = RVI32System::new();
        rv.bus.borrow_mut().write(0x1000_0005, 0xDEAD_BEEF);
    }

    #[test]
    fn test_instructions() {
        let mut rv = RVI32System::new();
        rv.reg_file.borrow_mut()[1] = 0x0102_0304;
        rv.reg_file.borrow_mut()[2] = 0x0203_0405;

        rv.bus.borrow_mut().rom.load(vec![
            0b000000000001_00001_000_00011_0010011,  // ADDI 1, r1, r3
            0b0000000_00001_00010_000_00100_0110011, // ADD r1, r2, r4
            0b0100000_00001_00010_000_00100_0110011, // SUB r1, r2, r4
        ]);

        rv.cycle();
        assert_eq!(*rv.state.borrow(), State::Decode);
        rv.cycle();
        assert_eq!(*rv.state.borrow(), State::Execute);
        rv.cycle();
        assert_eq!(*rv.state.borrow(), State::Fetch);
        assert_eq!(rv.stage_ex.borrow().get_alu_result_out(), 0x0102_0305);

        rv.cycle();
        assert_eq!(*rv.state.borrow(), State::Decode);
        rv.cycle();
        assert_eq!(*rv.state.borrow(), State::Execute);
        rv.cycle();
        assert_eq!(*rv.state.borrow(), State::Fetch);
        assert_eq!(rv.stage_ex.borrow().get_alu_result_out(), 0x0305_0709);

        rv.cycle();
        assert_eq!(*rv.state.borrow(), State::Decode);
        rv.cycle();
        assert_eq!(*rv.state.borrow(), State::Execute);
        rv.cycle();
        assert_eq!(*rv.state.borrow(), State::Fetch);
        assert_eq!(rv.stage_ex.borrow().get_alu_result_out(), 0x0101_0101);
    }
}
