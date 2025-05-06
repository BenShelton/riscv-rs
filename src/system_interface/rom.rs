use super::{MMIODevice, MMIOResult};

const ROM_SIZE: u32 = 1024 * 1024;
const ROM_SIZE_BYTES: usize = (ROM_SIZE / 4) as usize;
const ROM_MASK: u32 = (ROM_SIZE / 4) - 1;

pub struct RomDevice {
    rom: Vec<u32>,
}

impl RomDevice {
    pub fn new() -> Self {
        let rom = vec![0xFFFF_FFFF; ROM_SIZE_BYTES];
        Self { rom }
    }

    pub fn load(&mut self, data: Vec<u32>) {
        for i in 0..ROM_SIZE_BYTES {
            if i >= data.len() {
                self.rom[i] = 0xFFFF_FFFF;
            } else {
                self.rom[i] = data[i];
            }
        }
    }
}

impl Default for RomDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl MMIODevice for RomDevice {
    fn read_byte(&self, address: u32) -> MMIOResult<u8> {
        let index = ((address >> 2) & ROM_MASK) as usize;
        let value = self.rom[index];
        Ok((match address & 0b11 {
            0b00 => (value & 0xFF00_0000) >> 24,
            0b01 => (value & 0x00FF_0000) >> 16,
            0b10 => (value & 0x0000_FF00) >> 8,
            _ => value & 0x0000_00FF,
        }) as u8)
    }

    fn read_half_word(&self, address: u32) -> MMIOResult<u16> {
        let index = ((address >> 2) & ROM_MASK) as usize;
        let value = self.rom[index];
        Ok((match address & 0b10 {
            0 => (value & 0xFFFF_0000) >> 16,
            _ => value & 0x0000_FFFF,
        }) as u16)
    }

    fn read_word(&self, address: u32) -> MMIOResult<u32> {
        let index = ((address >> 2) & ROM_MASK) as usize;
        Ok(self.rom[index])
    }

    // Do nothing, you can't write to ROM
    fn write_byte(&mut self, _address: u32, _value: u8) -> MMIOResult<()> {
        Ok(())
    }
    fn write_half_word(&mut self, _address: u32, _value: u16) -> MMIOResult<()> {
        Ok(())
    }
    fn write_word(&mut self, _address: u32, _value: u32) -> MMIOResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_read() {
        let mut rom = RomDevice::new();
        rom.load(vec![0xDEAD_BEEF, 0xC0DE_CAFE]);
        assert_eq!(rom.read_word(0x0000_0000), Ok(0xDEAD_BEEF));
        assert_eq!(rom.read_word(0x0000_0004), Ok(0xC0DE_CAFE));
        assert_eq!(rom.read_word(0x0000_0008), Ok(0xFFFF_FFFF));
        assert_eq!(rom.read_half_word(0x0000_0000), Ok(0xDEAD));
        assert_eq!(rom.read_half_word(0x0000_0002), Ok(0xBEEF));
        assert_eq!(rom.read_half_word(0x0000_0004), Ok(0xC0DE));
        assert_eq!(rom.read_half_word(0x0000_0006), Ok(0xCAFE));
        assert_eq!(rom.read_half_word(0x0000_0008), Ok(0xFFFF));
        assert_eq!(rom.read_byte(0x0000_0000), Ok(0xDE));
        assert_eq!(rom.read_byte(0x0000_0001), Ok(0xAD));
        assert_eq!(rom.read_byte(0x0000_0002), Ok(0xBE));
        assert_eq!(rom.read_byte(0x0000_0003), Ok(0xEF));
        assert_eq!(rom.read_byte(0x0000_0004), Ok(0xC0));
        assert_eq!(rom.read_byte(0x0000_0005), Ok(0xDE));
        assert_eq!(rom.read_byte(0x0000_0006), Ok(0xCA));
        assert_eq!(rom.read_byte(0x0000_0007), Ok(0xFE));
        assert_eq!(rom.read_byte(0x0000_0008), Ok(0xFF));
    }

    #[test]
    fn test_write_does_nothing() {
        let mut rom = RomDevice::new();
        rom.write_word(0x0000_0000, 0xDEAD_BEEF).unwrap();
        rom.write_word(0x0000_0004, 0xC0DE_CAFE).unwrap();
        rom.write_half_word(0x0000_0000, 0xDEAD).unwrap();
        rom.write_half_word(0x0000_0002, 0xBEEF).unwrap();
        rom.write_byte(0x0000_0000, 0xDE).unwrap();
        rom.write_byte(0x0000_0001, 0xAD).unwrap();
        assert_eq!(rom.read_word(0x0000_0000), Ok(0xFFFF_FFFF));
        assert_eq!(rom.read_word(0x0000_0004), Ok(0xFFFF_FFFF));
        assert_eq!(rom.read_word(0x0000_0008), Ok(0xFFFF_FFFF));
    }

    #[test]
    fn test_read_wrap_around() {
        let mut rom = RomDevice::new();
        rom.rom[0] = 0xDEAD_BEEF;
        rom.rom[1] = 0xC0DE_CAFE;
        assert_eq!(rom.read_word(0x0010_0000), Ok(0xDEAD_BEEF));
        assert_eq!(rom.read_word(0x0010_0004), Ok(0xC0DE_CAFE));
        assert_eq!(rom.read_word(0x0010_0008), Ok(0xFFFF_FFFF));
        assert_eq!(rom.read_word(0x0040_0000), Ok(0xDEAD_BEEF));
        assert_eq!(rom.read_word(0x0040_0004), Ok(0xC0DE_CAFE));
        assert_eq!(rom.read_word(0x0040_0008), Ok(0xFFFF_FFFF));
        assert_eq!(rom.read_half_word(0x0010_0000), Ok(0xDEAD));
        assert_eq!(rom.read_half_word(0x0010_0002), Ok(0xBEEF));
        assert_eq!(rom.read_half_word(0x0010_0004), Ok(0xC0DE));
        assert_eq!(rom.read_half_word(0x0010_0006), Ok(0xCAFE));
        assert_eq!(rom.read_half_word(0x0010_0008), Ok(0xFFFF));
        assert_eq!(rom.read_half_word(0x0040_0000), Ok(0xDEAD));
        assert_eq!(rom.read_half_word(0x0040_0002), Ok(0xBEEF));
        assert_eq!(rom.read_half_word(0x0040_0004), Ok(0xC0DE));
        assert_eq!(rom.read_half_word(0x0040_0006), Ok(0xCAFE));
        assert_eq!(rom.read_half_word(0x0040_0008), Ok(0xFFFF));
        assert_eq!(rom.read_byte(0x0010_0000), Ok(0xDE));
        assert_eq!(rom.read_byte(0x0010_0001), Ok(0xAD));
        assert_eq!(rom.read_byte(0x0010_0002), Ok(0xBE));
        assert_eq!(rom.read_byte(0x0010_0003), Ok(0xEF));
        assert_eq!(rom.read_byte(0x0010_0004), Ok(0xC0));
        assert_eq!(rom.read_byte(0x0010_0005), Ok(0xDE));
        assert_eq!(rom.read_byte(0x0010_0006), Ok(0xCA));
        assert_eq!(rom.read_byte(0x0010_0007), Ok(0xFE));
        assert_eq!(rom.read_byte(0x0010_0008), Ok(0xFF));
        assert_eq!(rom.read_byte(0x0040_0000), Ok(0xDE));
        assert_eq!(rom.read_byte(0x0040_0001), Ok(0xAD));
        assert_eq!(rom.read_byte(0x0040_0002), Ok(0xBE));
        assert_eq!(rom.read_byte(0x0040_0003), Ok(0xEF));
        assert_eq!(rom.read_byte(0x0040_0004), Ok(0xC0));
        assert_eq!(rom.read_byte(0x0040_0005), Ok(0xDE));
        assert_eq!(rom.read_byte(0x0040_0006), Ok(0xCA));
        assert_eq!(rom.read_byte(0x0040_0007), Ok(0xFE));
        assert_eq!(rom.read_byte(0x0040_0008), Ok(0xFF));
    }
}
