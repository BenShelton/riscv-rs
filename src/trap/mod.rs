use crate::{csr::CSRInterface, utils::LatchValue};

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
    SetCSRJump,
    SetPc,
    ReturnFromTrap,
}

pub struct TrapParams<'a> {
    pub csr: &'a mut CSRInterface,
    pub begin_trap: bool,
    pub begin_trap_return: bool,
}

#[derive(Default, Debug)]
pub struct TrapInterface {
    pub state: LatchValue<TrapState>,
    pub mepc: LatchValue<u32>,
    pub mcause: LatchValue<u32>,
    pub mtval: LatchValue<u32>,
    pub return_to_pipeline_mode: LatchValue<bool>,
    pub set_pc: LatchValue<bool>,
    pub pc_to_set: LatchValue<u32>,
}

impl TrapInterface {
    pub fn new() -> Self {
        Self {
            state: LatchValue::new(TrapState::Idle),
            mepc: LatchValue::new(0),
            mcause: LatchValue::new(0),
            mtval: LatchValue::new(0),
            return_to_pipeline_mode: LatchValue::new(false),
            set_pc: LatchValue::new(false),
            pc_to_set: LatchValue::new(0),
        }
    }

    pub fn compute(&mut self, params: TrapParams) {
        if params.begin_trap {
            self.state.set(TrapState::SetCSRJump);
        } else if params.begin_trap_return {
            self.state.set(TrapState::ReturnFromTrap);
        } else {
            match self.state.get() {
                TrapState::Idle => {
                    self.return_to_pipeline_mode.set(false);
                    self.set_pc.set(false);
                }
                TrapState::SetCSRJump => {
                    let mcause = self.mcause.get();
                    params.csr.mepc = *self.mepc.get();
                    params.csr.mcause = *mcause;
                    params.csr.mtval = *self.mtval.get();

                    let index = mcause & 0x7FFF_FFFF;
                    let is_interrupt = (mcause & 0x8000_0000) != 0;
                    let offset = if is_interrupt { 0 } else { 48 };
                    self.pc_to_set
                        .set((params.csr.mtvec & 0xFFFF_FFFC) + offset + (index << 2));
                    self.set_pc.set(true);
                    self.return_to_pipeline_mode.set(true);
                    self.state.set(TrapState::Idle);
                }
                TrapState::SetPc => {
                    self.set_pc.set(true);
                    self.return_to_pipeline_mode.set(true);
                    self.state.set(TrapState::Idle);
                }
                TrapState::ReturnFromTrap => {
                    self.pc_to_set.set(params.csr.mepc);
                    self.state.set(TrapState::SetPc);
                }
            }
        }
    }

    pub fn latch_next(&mut self) {
        self.state.latch_next();
        self.mepc.latch_next();
        self.mcause.latch_next();
        self.mtval.latch_next();
        self.return_to_pipeline_mode.latch_next();
        self.set_pc.latch_next();
        self.pc_to_set.latch_next();
    }
}
