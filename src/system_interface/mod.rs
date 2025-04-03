mod ram;
mod rom;

pub use ram::RamDevice;
pub use rom::RomDevice;

pub trait MMIODevice {
    fn read(&self, address: u32) -> u32;
    fn write(&mut self, address: u32, value: u32);
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
    fn read(&self, address: u32) -> u32 {
        if address & 0b11 != 0 {
            panic!("Unaligned read from address {:#08X}", address);
        }

        if (address & PROGRAM_ROM_START) == PROGRAM_ROM_START {
            self.rom.read((address & 0x0FFF_FFFF) >> 2)
        } else if (address & RAM_START) == RAM_START {
            self.ram.read((address & 0x0FFF_FFFF) >> 2)
        } else {
            0
        }
    }

    fn write(&mut self, address: u32, value: u32) {
        if address & 0b11 != 0 {
            panic!(
                "Unaligned write to address {:#08X} (value={:#08X})",
                address, value
            );
        }

        if (address & RAM_START) == RAM_START {
            self.ram.write((address & 0x0FFF_FFFF) >> 2, value)
        }
    }
}
