mod ram;
mod rom;

pub use ram::RamDevice;
pub use rom::RomDevice;

#[derive(PartialEq, Eq, Debug)]
pub enum MMIOError {
    UnalignedRead(u32),
    UnalignedWrite(u32, u32),
}
impl std::fmt::Display for MMIOError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MMIOError::UnalignedRead(ref addr) => {
                write!(f, "Unaligned read from address {:#08X}", addr)
            }
            MMIOError::UnalignedWrite(addr, value) => {
                write!(
                    f,
                    "Unaligned write to address {:#08X} (value={:#08X})",
                    addr, value
                )
            }
        }
    }
}

type MMIOResult<T> = std::result::Result<T, MMIOError>;

pub trait MMIODevice {
    fn read_byte(&self, address: u32) -> MMIOResult<u8>;
    fn write_byte(&mut self, address: u32, value: u8) -> MMIOResult<()>;
    fn read_half_word(&self, address: u32) -> MMIOResult<u16>;
    fn write_half_word(&mut self, address: u32, value: u16) -> MMIOResult<()>;
    fn read_word(&self, address: u32) -> MMIOResult<u32>;
    fn write_word(&mut self, address: u32, value: u32) -> MMIOResult<()>;
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
    fn read_byte(&self, address: u32) -> MMIOResult<u8> {
        if (address & PROGRAM_ROM_START) == PROGRAM_ROM_START {
            self.rom.read_byte(address & 0x0FFF_FFFF)
        } else if (address & RAM_START) == RAM_START {
            self.ram.read_byte(address & 0x0FFF_FFFF)
        } else {
            Ok(0)
        }
    }

    fn read_half_word(&self, address: u32) -> MMIOResult<u16> {
        if address & 0b1 != 0 {
            return Err(MMIOError::UnalignedRead(address));
        }

        if (address & PROGRAM_ROM_START) == PROGRAM_ROM_START {
            self.rom.read_half_word(address & 0x0FFF_FFFF)
        } else if (address & RAM_START) == RAM_START {
            self.ram.read_half_word(address & 0x0FFF_FFFF)
        } else {
            Ok(0)
        }
    }

    fn read_word(&self, address: u32) -> MMIOResult<u32> {
        if address & 0b11 != 0 {
            return Err(MMIOError::UnalignedRead(address));
        }

        if (address & PROGRAM_ROM_START) == PROGRAM_ROM_START {
            self.rom.read_word(address & 0x0FFF_FFFF)
        } else if (address & RAM_START) == RAM_START {
            self.ram.read_word(address & 0x0FFF_FFFF)
        } else {
            Ok(0)
        }
    }

    fn write_byte(&mut self, address: u32, value: u8) -> MMIOResult<()> {
        if (address & RAM_START) == RAM_START {
            return self.ram.write_byte(address & 0x0FFF_FFFF, value);
        }

        Ok(())
    }

    fn write_half_word(&mut self, address: u32, value: u16) -> MMIOResult<()> {
        if address & 0b1 != 0 {
            return Err(MMIOError::UnalignedWrite(address, value as u32));
        }

        if (address & RAM_START) == RAM_START {
            return self.ram.write_half_word(address & 0x0FFF_FFFF, value);
        }

        Ok(())
    }

    fn write_word(&mut self, address: u32, value: u32) -> MMIOResult<()> {
        if address & 0b11 != 0 {
            return Err(MMIOError::UnalignedWrite(address, value));
        }

        if (address & RAM_START) == RAM_START {
            return self.ram.write_word(address & 0x0FFF_FFFF, value);
        }

        Ok(())
    }
}
