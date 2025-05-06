use super::{MMIODevice, MMIOResult};

const RAM_SIZE: u32 = 1024 * 1024 * 4;
const RAM_SIZE_BYTES: usize = (RAM_SIZE / 4) as usize;
const RAM_MASK: u32 = (RAM_SIZE / 4) - 1;

pub struct RamDevice {
    ram: Vec<u32>,
}

impl RamDevice {
    pub fn new() -> Self {
        let ram = vec![0xFFFF_FFFF; RAM_SIZE_BYTES];
        Self { ram }
    }
}

impl Default for RamDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl MMIODevice for RamDevice {
    fn read_byte(&self, address: u32) -> MMIOResult<u8> {
        let index = ((address >> 2) & RAM_MASK) as usize;
        let value = self.ram[index];
        Ok((match address & 0b11 {
            0b00 => (value & 0xFF00_0000) >> 24,
            0b01 => (value & 0x00FF_0000) >> 16,
            0b10 => (value & 0x0000_FF00) >> 8,
            _ => value & 0x0000_00FF,
        }) as u8)
    }

    fn read_half_word(&self, address: u32) -> MMIOResult<u16> {
        let index = ((address >> 2) & RAM_MASK) as usize;
        let value = self.ram[index];
        Ok((match address & 0b10 {
            0b0 => (value & 0xFFFF_0000) >> 16,
            _ => value & 0x0000_FFFF,
        }) as u16)
    }

    fn read_word(&self, address: u32) -> MMIOResult<u32> {
        let index = ((address >> 2) & RAM_MASK) as usize;
        Ok(self.ram[index])
    }

    fn write_byte(&mut self, address: u32, value: u8) -> MMIOResult<()> {
        let index = ((address >> 2) & RAM_MASK) as usize;
        let current_value = self.ram[index];
        self.ram[index] = match address & 0b11 {
            0b00 => (current_value & 0x00FF_FFFF) | ((value as u32) << 24),
            0b01 => (current_value & 0xFF00_FFFF) | ((value as u32) << 16),
            0b10 => (current_value & 0xFFFF_00FF) | ((value as u32) << 8),
            _ => (current_value & 0xFFFF_FF00) | (value as u32),
        };
        Ok(())
    }

    fn write_half_word(&mut self, address: u32, value: u16) -> MMIOResult<()> {
        let index = ((address >> 2) & RAM_MASK) as usize;
        let current_value = self.ram[index];
        self.ram[index] = match address & 0b10 {
            0b0 => (current_value & 0x0000_FFFF) | ((value as u32) << 16),
            _ => (current_value & 0xFFFF_0000) | (value as u32),
        };
        Ok(())
    }

    fn write_word(&mut self, address: u32, value: u32) -> MMIOResult<()> {
        let index = ((address >> 2) & RAM_MASK) as usize;
        self.ram[index] = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read() {
        let mut ram = RamDevice::new();
        ram.ram[0] = 0xDEAD_BEEF;
        ram.ram[1] = 0xC0DE_CAFE;
        assert_eq!(ram.read_word(0x0000_0000), Ok(0xDEAD_BEEF));
        assert_eq!(ram.read_word(0x0000_0004), Ok(0xC0DE_CAFE));
        assert_eq!(ram.read_word(0x0000_0008), Ok(0xFFFF_FFFF));
        assert_eq!(ram.read_half_word(0x0000_0000), Ok(0xDEAD));
        assert_eq!(ram.read_half_word(0x0000_0002), Ok(0xBEEF));
        assert_eq!(ram.read_half_word(0x0000_0004), Ok(0xC0DE));
        assert_eq!(ram.read_half_word(0x0000_0006), Ok(0xCAFE));
        assert_eq!(ram.read_half_word(0x0000_0008), Ok(0xFFFF));
        assert_eq!(ram.read_byte(0x0000_0000), Ok(0xDE));
        assert_eq!(ram.read_byte(0x0000_0001), Ok(0xAD));
        assert_eq!(ram.read_byte(0x0000_0002), Ok(0xBE));
        assert_eq!(ram.read_byte(0x0000_0003), Ok(0xEF));
        assert_eq!(ram.read_byte(0x0000_0004), Ok(0xC0));
        assert_eq!(ram.read_byte(0x0000_0005), Ok(0xDE));
        assert_eq!(ram.read_byte(0x0000_0006), Ok(0xCA));
        assert_eq!(ram.read_byte(0x0000_0007), Ok(0xFE));
        assert_eq!(ram.read_byte(0x0000_0008), Ok(0xFF));
    }

    #[test]
    fn test_write() {
        let mut ram = RamDevice::new();
        ram.write_word(0x0000_0000, 0xDEAD_BEEF).unwrap();
        ram.write_word(0x0000_0004, 0xC0DE_CAFE).unwrap();
        assert_eq!(ram.read_word(0x0000_0000), Ok(0xDEAD_BEEF));
        assert_eq!(ram.read_word(0x0000_0004), Ok(0xC0DE_CAFE));
        assert_eq!(ram.read_word(0x0000_0008), Ok(0xFFFF_FFFF));

        ram.write_half_word(0x0000_0000, 0xABAD).unwrap();
        ram.write_half_word(0x0000_0006, 0x1DEA).unwrap();
        assert_eq!(ram.read_word(0x0000_0000), Ok(0xABAD_BEEF));
        assert_eq!(ram.read_word(0x0000_0004), Ok(0xC0DE_1DEA));

        ram.write_byte(0x0000_0000, 0xAA).unwrap();
        ram.write_byte(0x0000_0003, 0xBB).unwrap();
        ram.write_byte(0x0000_0007, 0xCC).unwrap();
        assert_eq!(ram.read_word(0x0000_0000), Ok(0xAAAD_BEBB));
        assert_eq!(ram.read_word(0x0000_0004), Ok(0xC0DE_1DCC));
    }

    #[test]
    fn test_write_wrap_around() {
        let mut ram = RamDevice::new();
        ram.write_word(0x1000_0000, 0xDEAD_BEEF).unwrap();
        ram.write_word(0x1000_0004, 0xC0DE_CAFE).unwrap();
        assert_eq!(ram.read_word(0x0000_0000), Ok(0xDEAD_BEEF));
        assert_eq!(ram.read_word(0x0000_0004), Ok(0xC0DE_CAFE));
        assert_eq!(ram.read_word(0x0000_0008), Ok(0xFFFF_FFFF));

        ram.write_half_word(0x1000_0000, 0xABAD).unwrap();
        ram.write_half_word(0x1000_0006, 0x1DEA).unwrap();
        assert_eq!(ram.read_word(0x0000_0000), Ok(0xABAD_BEEF));
        assert_eq!(ram.read_word(0x0000_0004), Ok(0xC0DE_1DEA));

        ram.write_byte(0x1000_0000, 0xAA).unwrap();
        ram.write_byte(0x1000_0003, 0xBB).unwrap();
        ram.write_byte(0x1000_0007, 0xCC).unwrap();
        assert_eq!(ram.read_word(0x0000_0000), Ok(0xAAAD_BEBB));
        assert_eq!(ram.read_word(0x0000_0004), Ok(0xC0DE_1DCC));
    }

    #[test]
    fn test_read_wrap_around() {
        let mut ram = RamDevice::new();
        ram.ram[0] = 0xDEAD_BEEF;
        ram.ram[1] = 0xC0DE_CAFE;
        assert_eq!(ram.read_word(0x1000_0000), Ok(0xDEAD_BEEF));
        assert_eq!(ram.read_word(0x1000_0004), Ok(0xC0DE_CAFE));
        assert_eq!(ram.read_word(0x1000_0008), Ok(0xFFFF_FFFF));
        assert_eq!(ram.read_word(0x4000_0000), Ok(0xDEAD_BEEF));
        assert_eq!(ram.read_word(0x4000_0004), Ok(0xC0DE_CAFE));
        assert_eq!(ram.read_word(0x4000_0008), Ok(0xFFFF_FFFF));
        assert_eq!(ram.read_half_word(0x1000_0000), Ok(0xDEAD));
        assert_eq!(ram.read_half_word(0x1000_0002), Ok(0xBEEF));
        assert_eq!(ram.read_half_word(0x1000_0004), Ok(0xC0DE));
        assert_eq!(ram.read_half_word(0x1000_0006), Ok(0xCAFE));
        assert_eq!(ram.read_half_word(0x1000_0008), Ok(0xFFFF));
        assert_eq!(ram.read_half_word(0x4000_0000), Ok(0xDEAD));
        assert_eq!(ram.read_half_word(0x4000_0002), Ok(0xBEEF));
        assert_eq!(ram.read_half_word(0x4000_0004), Ok(0xC0DE));
        assert_eq!(ram.read_half_word(0x4000_0006), Ok(0xCAFE));
        assert_eq!(ram.read_half_word(0x4000_0008), Ok(0xFFFF));
        assert_eq!(ram.read_byte(0x1000_0000), Ok(0xDE));
        assert_eq!(ram.read_byte(0x1000_0001), Ok(0xAD));
        assert_eq!(ram.read_byte(0x1000_0002), Ok(0xBE));
        assert_eq!(ram.read_byte(0x1000_0003), Ok(0xEF));
        assert_eq!(ram.read_byte(0x1000_0004), Ok(0xC0));
        assert_eq!(ram.read_byte(0x1000_0005), Ok(0xDE));
        assert_eq!(ram.read_byte(0x1000_0006), Ok(0xCA));
        assert_eq!(ram.read_byte(0x1000_0007), Ok(0xFE));
        assert_eq!(ram.read_byte(0x1000_0008), Ok(0xFF));
        assert_eq!(ram.read_byte(0x4000_0000), Ok(0xDE));
        assert_eq!(ram.read_byte(0x4000_0001), Ok(0xAD));
        assert_eq!(ram.read_byte(0x4000_0002), Ok(0xBE));
        assert_eq!(ram.read_byte(0x4000_0003), Ok(0xEF));
        assert_eq!(ram.read_byte(0x4000_0004), Ok(0xC0));
        assert_eq!(ram.read_byte(0x4000_0005), Ok(0xDE));
        assert_eq!(ram.read_byte(0x4000_0006), Ok(0xCA));
        assert_eq!(ram.read_byte(0x4000_0007), Ok(0xFE));
        assert_eq!(ram.read_byte(0x4000_0008), Ok(0xFF));
    }
}
