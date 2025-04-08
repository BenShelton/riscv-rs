use super::{LatchValue, PipelineStage, execute::ExecutionValue};

#[derive(Debug, PartialEq, Eq)]
pub struct MemoryAccessValue {
    pub alu_result: u32,
    pub rd: u8,
    pub is_alu_operation: bool,
}

pub struct InstructionMemoryAccess {
    alu_result: LatchValue<u32>,
    rd: LatchValue<u8>,
    is_alu_operation: LatchValue<bool>,
    should_stall: Box<dyn Fn() -> bool>,
    get_execution_value_in: Box<dyn Fn() -> ExecutionValue>,
}

pub struct InstructionMemoryAccessParams {
    pub should_stall: Box<dyn Fn() -> bool>,
    pub get_execution_value_in: Box<dyn Fn() -> ExecutionValue>,
}

impl InstructionMemoryAccess {
    pub fn new(params: InstructionMemoryAccessParams) -> Self {
        Self {
            should_stall: params.should_stall,
            get_execution_value_in: params.get_execution_value_in,
            alu_result: LatchValue::new(0),
            rd: LatchValue::new(0),
            is_alu_operation: LatchValue::new(false),
        }
    }

    pub fn get_memory_access_value_out(&self) -> MemoryAccessValue {
        MemoryAccessValue {
            alu_result: *self.alu_result.get(),
            rd: *self.rd.get(),
            is_alu_operation: *self.is_alu_operation.get(),
        }
    }
}

impl PipelineStage for InstructionMemoryAccess {
    fn compute(&mut self) {
        if (self.should_stall)() {
            return;
        }
        let execution_value = (self.get_execution_value_in)();
        self.alu_result.set(execution_value.alu_result);
        self.rd.set(execution_value.rd);
        self.is_alu_operation.set(execution_value.is_alu_operation);
    }

    fn latch_next(&mut self) {
        self.alu_result.latch_next();
        self.rd.latch_next();
        self.is_alu_operation.latch_next();
    }
}
