mod ram;
mod rom;

pub use ram::RamDevice;
pub use rom::RomDevice;

pub trait MMIODevice {
    fn read_byte(&self, address: u32) -> u8;
    fn write_byte(&mut self, address: u32, value: u8);
    fn read_half_word(&self, address: u32) -> u16;
    fn write_half_word(&mut self, address: u32, value: u16);
    fn read_word(&self, address: u32) -> u32;
    fn write_word(&mut self, address: u32, value: u32);
}

pub const PROGRAM_ROM_START: u32 = 0x1000_0000;
pub const PROGRAM_ROM_END: u32 = 0x1FFF_FFFF;
pub const RAM_START: u32 = 0x2000_0000;
pub const RAM_END: u32 = 0x2FFF_FFFF;

pub struct SystemInterface {
    pub rom: RomDevice,
    pub ram: RamDevice,
}

impl SystemInterface {
    pub fn new(rom: RomDevice, ram: RamDevice) -> Self {
        Self { rom, ram }
    }
}

impl MMIODevice for SystemInterface {
    fn read_byte(&self, address: u32) -> u8 {
        // if address & 0b11 != 0 {
        //     panic!("Unaligned read from address {:#08X}", address);
        // }

        if (address & PROGRAM_ROM_START) == PROGRAM_ROM_START {
            self.rom.read_byte(address & 0x0FFF_FFFF)
        } else if (address & RAM_START) == RAM_START {
            self.ram.read_byte(address & 0x0FFF_FFFF)
        } else {
            0
        }
    }

    fn read_half_word(&self, address: u32) -> u16 {
        // if address & 0b11 != 0 {
        //     panic!("Unaligned read from address {:#08X}", address);
        // }

        if (address & PROGRAM_ROM_START) == PROGRAM_ROM_START {
            self.rom.read_half_word(address & 0x0FFF_FFFF)
        } else if (address & RAM_START) == RAM_START {
            self.ram.read_half_word(address & 0x0FFF_FFFF)
        } else {
            0
        }
    }

    fn read_word(&self, address: u32) -> u32 {
        if address & 0b11 != 0 {
            panic!("Unaligned read from address {:#08X}", address);
        }

        if (address & PROGRAM_ROM_START) == PROGRAM_ROM_START {
            self.rom.read_word(address & 0x0FFF_FFFF)
        } else if (address & RAM_START) == RAM_START {
            self.ram.read_word(address & 0x0FFF_FFFF)
        } else {
            0
        }
    }

    fn write_byte(&mut self, address: u32, value: u8) {
        // if address & 0b11 != 0 {
        //     panic!(
        //         "Unaligned write to address {:#08X} (value={:#08X})",
        //         address, value
        //     );
        // }

        if (address & RAM_START) == RAM_START {
            self.ram.write_byte(address & 0x0FFF_FFFF, value)
        }
    }

    fn write_half_word(&mut self, address: u32, value: u16) {
        // if address & 0b11 != 0 {
        //     panic!(
        //         "Unaligned write to address {:#08X} (value={:#08X})",
        //         address, value
        //     );
        // }

        if (address & RAM_START) == RAM_START {
            self.ram.write_half_word(address & 0x0FFF_FFFF, value)
        }
    }

    fn write_word(&mut self, address: u32, value: u32) {
        if address & 0b11 != 0 {
            panic!(
                "Unaligned write to address {:#08X} (value={:#08X})",
                address, value
            );
        }

        if (address & RAM_START) == RAM_START {
            self.ram.write_word(address & 0x0FFF_FFFF, value)
        }
    }
}
