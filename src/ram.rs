const RAM_SIZE: u32 = 1024 * 1024 * 4;
const RAM_SIZE_BYTES: usize = (RAM_SIZE / 4) as usize;
const RAM_MASK: u32 = (RAM_SIZE / 4) - 1;

pub struct RamDevice {
    ram: Vec<u32>,
}

impl RamDevice {
    pub fn new() -> Self {
        let mut ram = Vec::with_capacity(RAM_SIZE_BYTES);
        for _ in 0..RAM_SIZE_BYTES {
            ram.push(0xFFFF_FFFF);
        }
        RamDevice { ram }
    }
}

impl crate::system_interface::MMIODevice for RamDevice {
    fn read(&self, address: u32) -> u32 {
        let index = (address & RAM_MASK) as usize;
        self.ram[index]
    }

    fn write(&mut self, address: u32, value: u32) {
        let index = (address & RAM_MASK) as usize;
        self.ram[index] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_interface::MMIODevice;

    #[test]
    fn test_read() {
        let mut ram = RamDevice::new();
        ram.ram[0] = 0xDEAD_BEEF;
        ram.ram[1] = 0xC0DE_CAFE;
        assert_eq!(ram.read(0x0000_0000), 0xDEAD_BEEF);
        assert_eq!(ram.read(0x0000_0001), 0xC0DE_CAFE);
        assert_eq!(ram.read(0x0000_0002), 0xFFFF_FFFF);
    }

    #[test]
    fn test_write() {
        let mut ram = RamDevice::new();
        ram.write(0x0000_0000, 0xDEAD_BEEF);
        ram.write(0x0000_0001, 0xC0DE_CAFE);
        assert_eq!(ram.read(0x0000_0000), 0xDEAD_BEEF);
        assert_eq!(ram.read(0x0000_0001), 0xC0DE_CAFE);
        assert_eq!(ram.read(0x0000_0002), 0xFFFF_FFFF);
    }

    #[test]
    fn test_write_wrap_around() {
        let mut ram = RamDevice::new();
        ram.write(0x0010_0000, 0xDEAD_BEEF);
        ram.write(0x0010_0001, 0xC0DE_CAFE);
        assert_eq!(ram.read(0x0000_0000), 0xDEAD_BEEF);
        assert_eq!(ram.read(0x0000_0001), 0xC0DE_CAFE);
    }
}
