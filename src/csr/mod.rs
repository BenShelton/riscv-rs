use crate::utils::LatchValue;

pub const CSR_OPERATION_RW: u8 = 0b001;
pub const CSR_OPERATION_RS: u8 = 0b010;
pub const CSR_OPERATION_RC: u8 = 0b011;
pub const CSR_OPERATION_RWI: u8 = 0b101;
pub const CSR_OPERATION_RSI: u8 = 0b110;
pub const CSR_OPERATION_RCI: u8 = 0b111;

#[derive(Default)]
pub struct CSRInterface {
    pub cycles: LatchValue<u64>,
    pub instret: LatchValue<u64>,
}

impl CSRInterface {
    pub fn new() -> Self {
        CSRInterface {
            cycles: LatchValue::new(0),
            instret: LatchValue::new(0),
        }
    }

    pub fn read(&self, address: u32) -> u32 {
        let permission = (address >> 8) & 0b11;

        if permission != 0 {
            panic!("CSR Read: Only user mode implemented");
        }

        match address {
            0xC00 => *self.cycles.get() as u32,
            0xC01 => *self.cycles.get() as u32,
            0xC02 => *self.instret.get() as u32,
            0xC80 => (*self.cycles.get() >> 32) as u32,
            0xC81 => (*self.cycles.get() >> 32) as u32,
            0xC82 => (*self.instret.get() >> 32) as u32,
            _ => 0,
        }
    }

    pub fn write(&self, address: u32, _value: u32) {
        let is_read_only = address >> 10;
        let permission = (address >> 8) & 0b11;

        if permission != 0 {
            panic!("CSR Write: Only user mode implemented");
        }

        if is_read_only != 0 {
            panic!("CSR Write: Attempt to write a read-only register");
        }
    }

    pub fn compute(&mut self) {
        self.cycles.set(self.cycles.get() + 1);
    }

    pub fn latch_next(&mut self) {
        self.cycles.latch_next();
        self.instret.latch_next();
    }
}
