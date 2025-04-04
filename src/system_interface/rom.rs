use super::MMIODevice;

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

impl MMIODevice for RomDevice {
    fn read(&self, address: u32) -> u32 {
        let index = (address & ROM_MASK) as usize;
        self.rom[index]
    }

    fn write(&mut self, _address: u32, _value: u32) {
        // Do nothing, you can't write to ROM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_read() {
        let mut rom = RomDevice::new();
        rom.load(vec![0xDEAD_BEEF, 0xC0DE_CAFE]);
        assert_eq!(rom.read(0x0000_0000), 0xDEAD_BEEF);
        assert_eq!(rom.read(0x0000_0001), 0xC0DE_CAFE);
        assert_eq!(rom.read(0x0000_0002), 0xFFFF_FFFF);
    }

    #[test]
    fn test_write_does_nothing() {
        let mut rom = RomDevice::new();
        rom.write(0x0000_0000, 0xDEAD_BEEF);
        rom.write(0x0000_0001, 0xC0DE_CAFE);
        assert_eq!(rom.read(0x0000_0000), 0xFFFF_FFFF);
        assert_eq!(rom.read(0x0000_0001), 0xFFFF_FFFF);
        assert_eq!(rom.read(0x0000_0002), 0xFFFF_FFFF);
    }

    #[test]
    fn test_read_wrap_around() {
        let mut rom = RomDevice::new();
        rom.rom[0] = 0xDEAD_BEEF;
        rom.rom[1] = 0xC0DE_CAFE;
        assert_eq!(rom.read(0x0010_0000), 0xDEAD_BEEF);
        assert_eq!(rom.read(0x0010_0001), 0xC0DE_CAFE);
        assert_eq!(rom.read(0x0010_0002), 0xFFFF_FFFF);
        assert_eq!(rom.read(0x0040_0000), 0xDEAD_BEEF);
        assert_eq!(rom.read(0x0040_0001), 0xC0DE_CAFE);
        assert_eq!(rom.read(0x0040_0002), 0xFFFF_FFFF);
    }
}
