use crate::{
    csr::CSRInterface,
    system_interface::{MMIODevice, SystemInterface},
    utils::LatchValue,
};

pub const MCAUSE_USER_SOFTWARE_INTERRUPT: u32 = 0x8000_0000;
pub const MCAUSE_SUPERVISOR_SOFTWARE_INTERRUPT: u32 = 0x8000_0001;
pub const MCAUSE_RESERVED_0: u32 = 0x8000_0002;
pub const MCAUSE_MACHINE_SOFTWARE_INTERRUPT: u32 = 0x8000_0003;
pub const MCAUSE_USER_TIMER_INTERRUPT: u32 = 0x8000_0004;
pub const MCAUSE_SUPERVISOR_TIMER_INTERRUPT: u32 = 0x8000_0005;
pub const MCAUSE_RESERVED_1: u32 = 0x8000_0006;
pub const MCAUSE_MACHINE_TIMER_INTERRUPT: u32 = 0x8000_0007;
pub const MCAUSE_USER_EXTERNAL_INTERRUPT: u32 = 0x8000_0008;
pub const MCAUSE_SUPERVISOR_EXTERNAL_INTERRUPT: u32 = 0x8000_0009;
pub const MCAUSE_RESERVED_2: u32 = 0x8000_000A;
pub const MCAUSE_MACHINE_EXTERNAL_INTERRUPT: u32 = 0x8000_000B;

pub const MCAUSE_INSTRUCTION_ADDRESS_MISALIGNED: u32 = 0x0000_0000;
pub const MCAUSE_INSTRUCTION_ACCESS_FAULT: u32 = 0x0000_0001;
pub const MCAUSE_ILLEGAL_INSTRUCTION: u32 = 0x0000_0002;
pub const MCAUSE_BREAKPOINT: u32 = 0x0000_0003;
pub const MCAUSE_LOAD_ADDRESS_MISALIGNED: u32 = 0x0000_0004;
pub const MCAUSE_LOAD_ACCESS_FAULT: u32 = 0x0000_0005;
pub const MCAUSE_STORE_AMO_ADDRESS_MISALIGNED: u32 = 0x0000_0006;
pub const MCAUSE_STORE_AMO_ACCESS_FAULT: u32 = 0x0000_0007;
pub const MCAUSE_ENVIRONMENT_CALL_FROM_UMODE: u32 = 0x0000_0008;
pub const MCAUSE_ENVIRONMENT_CALL_FROM_SMODE: u32 = 0x0000_0009;
pub const MCAUSE_RESERVED_3: u32 = 0x0000_000A;
pub const MCAUSE_ENVIRONMENT_CALL_FROM_MMODE: u32 = 0x0000_000B;
pub const MCAUSE_INSTRUCTION_PAGE_FAULT: u32 = 0x0000_000C;
pub const MCAUSE_LOAD_PAGE_FAULT: u32 = 0x0000_000D;
pub const MCAUSE_RESERVED_4: u32 = 0x0000_000E;
pub const MCAUSE_STORE_AMO_PAGE_FAULT: u32 = 0x0000_000F;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TrapState {
    #[default]
    Idle,
    SetCSRLoadJump,
    SetPc,
    ReturnFromTrap,
}

pub struct TrapParams<'a> {
    pub should_stall: bool,
    pub csr: &'a mut CSRInterface,
    pub bus: &'a mut SystemInterface,
    pub set_pc: Box<dyn FnOnce(u32) + 'a>,
    pub return_to_pipeline_mode: Box<dyn FnOnce() + 'a>,
}

#[derive(Default, Debug)]
pub struct TrapInterface {
    pub state: LatchValue<TrapState>,
    pub mepc: LatchValue<u32>,
    pub mcause: LatchValue<u32>,
    pub mtval: LatchValue<u32>,
    pc_to_set: LatchValue<u32>,
}

impl TrapInterface {
    pub fn new() -> Self {
        Self {
            state: LatchValue::new(TrapState::Idle),
            mepc: LatchValue::new(0),
            mcause: LatchValue::new(0),
            mtval: LatchValue::new(0),
            pc_to_set: LatchValue::new(0),
        }
    }

    pub fn trap_exception(&mut self, mepc: u32, mcause: u32, mtval: u32) {
        self.mepc.set(mepc);
        self.mcause.set(mcause);
        self.mtval.set(mtval);
        self.state.set(TrapState::SetCSRLoadJump);
    }

    pub fn compute(&mut self, params: TrapParams) {
        if params.should_stall {
            return;
        }

        match self.state.get() {
            TrapState::Idle => {}
            TrapState::SetCSRLoadJump => {
                let mcause = self.mcause.get();
                params.csr.mepc = *self.mepc.get();
                params.csr.mcause = *mcause;
                params.csr.mtval = *self.mtval.get();

                let index = mcause & 0x7FFF_FFFF;
                let is_interrupt = (mcause & 0x8000_0000) != 0;
                let offset = if is_interrupt { 0 } else { 48 };
                let addr = params.csr.mtvec + offset + (index << 2);

                self.pc_to_set.set(params.bus.read_word(addr).unwrap());
                self.state.set(TrapState::SetPc);
            }
            TrapState::SetPc => {
                (params.set_pc)(*self.pc_to_set.get());
                (params.return_to_pipeline_mode)();
                self.state.set(TrapState::Idle);
            }
            TrapState::ReturnFromTrap => {}
        }
    }

    pub fn latch_next(&mut self) {
        self.state.latch_next();
        self.mepc.latch_next();
        self.mcause.latch_next();
        self.mtval.latch_next();
        self.pc_to_set.latch_next();
    }
}
