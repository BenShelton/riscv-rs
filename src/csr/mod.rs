use crate::utils::LatchValue;

pub const CSR_OPERATION_RW: u8 = 0b001;
pub const CSR_OPERATION_RS: u8 = 0b010;
pub const CSR_OPERATION_RC: u8 = 0b011;
pub const CSR_OPERATION_RWI: u8 = 0b101;
pub const CSR_OPERATION_RSI: u8 = 0b110;
pub const CSR_OPERATION_RCI: u8 = 0b111;

pub const CSRM_MODE_MISA: u32 = 0x301;
pub const CSRM_MODE_MVENDORID: u32 = 0xF11;
pub const CSRM_MODE_MARCHID: u32 = 0xF12;
pub const CSRM_MODE_MIMPID: u32 = 0xF13;
pub const CSRM_MODE_MHARTID: u32 = 0xF14;
pub const CSRM_MODE_MSTATUS: u32 = 0x300;
pub const CSRM_MODE_MTVEC: u32 = 0x305;
pub const CSRM_MODE_MIE: u32 = 0x304;
pub const CSRM_MODE_MIP: u32 = 0x344;
pub const CSRM_MODE_MCAUSE: u32 = 0x342;
pub const CSRM_MODE_MEPC: u32 = 0x341;
pub const CSRM_MODE_MSCRATCH: u32 = 0x340;
pub const CSRM_MODE_MTVAL: u32 = 0x343;

pub const MSTATUS_MASK: u32 = (1 << 3) | (1 << 7);

#[derive(Default)]
pub struct CSRInterface {
    pub cycles: LatchValue<u64>,
    pub instret: LatchValue<u64>,
    /// Encodes CPU capabilities, top 2 bits encode width (XLEN), bottom 26 encode extensions
    misa: u32,
    /// JEDEC manufacturer ID
    mvendorid: u32,
    /// Microarchitecture ID
    marchid: u32,
    /// Processor version
    mimpid: u32,
    /// Hart ID
    mhartid: u32,
    /// Various specific flags and settings, including global interrupt enable, and a lot of noop bits (for us)
    mstatus: u32,
    /// Encodes the base trap vector address + mode (table or single handler)
    pub mtvec: u32,
    /// Interrupt enable / disable
    mie: u32,
    /// Interrupt-pending
    mip: u32,
    /// Trap cause. Top bit set = interrupt, reset = exception - reset indicates the type
    pub mcause: u32,
    /// Exception Program Counter
    pub mepc: u32,
    /// General use reg for M-Mode, mostly used to hold a pointer context space apparently
    mscratch: u32,
    /// Trap-value register, can hold the address of a faulting instruction
    pub mtval: u32,
    // (Not a CSR) Memory-mapped 64-bit reg, with a writable value. When mtime == mtimecmp, a timer interrupt fires
    mtimecmp: LatchValue<u64>,
}

impl CSRInterface {
    pub fn new() -> Self {
        CSRInterface {
            cycles: LatchValue::new(0),
            instret: LatchValue::new(0),
            misa: 0x4000_0100,
            mvendorid: 0,
            marchid: 0,
            mimpid: 0,
            mhartid: 0,
            mstatus: 0,
            mtvec: 0x1000_0004,
            mie: 0x0000_0888,
            mip: 0,
            mcause: 0,
            mepc: 0,
            mscratch: 0,
            mtval: 0,
            mtimecmp: LatchValue::new(0),
        }
    }

    pub fn read(&self, address: u32) -> u32 {
        match address {
            // User level
            0xC00 => *self.cycles.get() as u32,
            0xC01 => *self.cycles.get() as u32,
            0xC02 => *self.instret.get() as u32,
            0xC80 => (*self.cycles.get() >> 32) as u32,
            0xC81 => (*self.cycles.get() >> 32) as u32,
            0xC82 => (*self.instret.get() >> 32) as u32,
            // Machine mode
            CSRM_MODE_MISA => self.misa,
            CSRM_MODE_MVENDORID => self.mvendorid,
            CSRM_MODE_MARCHID => self.marchid,
            CSRM_MODE_MIMPID => self.mimpid,
            CSRM_MODE_MHARTID => self.mhartid,
            CSRM_MODE_MSTATUS => self.mstatus,
            CSRM_MODE_MTVEC => self.mtvec,
            CSRM_MODE_MIE => self.mie,
            CSRM_MODE_MIP => self.mip,
            CSRM_MODE_MCAUSE => self.mcause,
            CSRM_MODE_MEPC => self.mepc,
            CSRM_MODE_MSCRATCH => self.mscratch,
            CSRM_MODE_MTVAL => self.mtval,
            _ => {
                panic!("Unknown CSR: {:#08X}", address & 0b111)
            }
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        let is_read_only = address >> 10;

        if is_read_only != 0 {
            panic!("CSR Write: Attempt to write a read-only register");
        }

        match address {
            CSRM_MODE_MSTATUS => self.mstatus = value & MSTATUS_MASK,
            CSRM_MODE_MIE => self.mie = value,
            CSRM_MODE_MIP => self.mip = value,
            CSRM_MODE_MCAUSE => self.mcause = value,
            CSRM_MODE_MEPC => self.mepc = value,
            CSRM_MODE_MSCRATCH => self.mscratch = value,
            CSRM_MODE_MTVAL => self.mtval = value,
            _ => {}
        }
    }

    pub fn compute(&mut self) {
        self.cycles.set(self.cycles.get() + 1);
    }

    pub fn latch_next(&mut self) {
        self.cycles.latch_next();
        self.instret.latch_next();
        self.mtimecmp.latch_next();
    }
}
