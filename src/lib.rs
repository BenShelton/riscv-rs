#![allow(dead_code)]

mod pipeline;
mod ram;
mod rom;
mod system_interface;

use ram::RamDevice;
use rom::RomDevice;
use system_interface::SystemInterface;

struct RVI32System {
    bus: SystemInterface,
}

impl RVI32System {
    pub fn new() -> Self {
        let rom = RomDevice::new();
        let ram = RamDevice::new();
        let bus = SystemInterface::new(rom, ram);
        RVI32System { bus }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_interface::MMIODevice;

    #[test]
    fn test_rom_read() {
        let mut rv = RVI32System::new();
        rv.bus.rom.load(vec![0xDEAD_BEEF, 0xC0DE_CAFE]);
        assert_eq!(rv.bus.read(0x1000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.read(0x1000_0004), 0xC0DE_CAFE);
        assert_eq!(rv.bus.read(0x1000_0008), 0xFFFF_FFFF);
    }

    #[test]
    fn test_rom_write_does_nothing() {
        let mut rv = RVI32System::new();
        rv.bus.write(0x1000_0000, 0xDEAD_BEEF);
        rv.bus.write(0x1000_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.read(0x1000_0000), 0xFFFF_FFFF);
        assert_eq!(rv.bus.read(0x1000_0004), 0xFFFF_FFFF);
    }

    #[test]
    fn test_ram_write_read() {
        let mut rv = RVI32System::new();
        rv.bus.write(0x2000_0000, 0xDEAD_BEEF);
        rv.bus.write(0x2000_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.read(0x2000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.read(0x2000_0004), 0xC0DE_CAFE);
    }

    #[test]
    fn test_ram_write_wrap_around() {
        let mut rv = RVI32System::new();
        rv.bus.write(0x2040_0000, 0xDEAD_BEEF);
        rv.bus.write(0x2040_0004, 0xC0DE_CAFE);
        assert_eq!(rv.bus.read(0x2000_0000), 0xDEAD_BEEF);
        assert_eq!(rv.bus.read(0x2000_0004), 0xC0DE_CAFE);
    }

    #[test]
    #[should_panic(expected = "Unaligned read from address 0x10000005")]
    fn test_panic_on_misaligned_read() {
        let rv = RVI32System::new();
        rv.bus.read(0x1000_0005);
    }

    #[test]
    #[should_panic(expected = "Unaligned write to address 0x10000005 (value=0xDEADBEEF)")]
    fn test_panic_on_misaligned_write() {
        let mut rv = RVI32System::new();
        rv.bus.write(0x1000_0005, 0xDEAD_BEEF);
    }
}
