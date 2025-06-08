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

pub const MSTATUS_MIE_BIT: u32 = 3;
pub const MSTATUS_MIE_MASK: u32 = 1 << MSTATUS_MIE_BIT;
pub const MSTATUS_MPIE_BIT: u32 = 7;
pub const MSTATUS_MPIE_MASK: u32 = 1 << MSTATUS_MPIE_BIT;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TrapState {
    #[default]
    Idle,
    SetCSRJump,
    SetPc,
    ReturnFromTrap,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PipelineTrapParams {
    pub mepc: u32,
    pub mcause: u32,
    pub mtval: u32,
    pub trap: bool,
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
    pub flush: LatchValue<bool>,
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
            flush: LatchValue::new(false),
        }
    }

    pub fn compute(&mut self, params: TrapParams) {
        if params.begin_trap {
            self.state.set(TrapState::SetCSRJump);
            self.flush.set(true);
        } else if params.begin_trap_return {
            self.state.set(TrapState::ReturnFromTrap);
            self.flush.set(false);
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

                    // move the current MIE to MPIE
                    let mie = (params.csr.mstatus & MSTATUS_MIE_MASK) >> MSTATUS_MIE_BIT;
                    // unset the MPIE bit
                    params.csr.mstatus &= !MSTATUS_MPIE_MASK;
                    // set MPIE to the current MIE
                    params.csr.mstatus |= mie << MSTATUS_MPIE_BIT;
                    // unset MIE
                    params.csr.mstatus &= !MSTATUS_MIE_MASK;

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

                    // move the current MPIE to MIE
                    let mpie = (params.csr.mstatus & MSTATUS_MPIE_MASK) >> MSTATUS_MPIE_BIT;
                    // unset the MIE bit
                    params.csr.mstatus &= !MSTATUS_MIE_MASK;
                    // set MIE to the current MPIE
                    params.csr.mstatus |= mpie << MSTATUS_MIE_BIT;
                    // unset MPIE
                    params.csr.mstatus &= !MSTATUS_MPIE_MASK;
                }
            }
            self.flush.set(false);
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
        self.flush.latch_next();
    }
}
