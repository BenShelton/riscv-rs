use super::{PipelineStage, execute::ExecutionValue};

#[derive(Debug, PartialEq, Eq)]
pub struct MemoryAccessValue {
    pub alu_result: u32,
    pub rd: u8,
    pub is_alu_operation: bool,
}

pub struct InstructionMemoryAccess {
    should_stall: Box<dyn Fn() -> bool>,
    get_execution_value_in: Box<dyn Fn() -> ExecutionValue>,
    alu_result: u32,
    alu_result_next: u32,
    rd: u8,
    rd_next: u8,
    is_alu_operation: bool,
    is_alu_operation_next: bool,
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
            alu_result: 0,
            alu_result_next: 0,
            rd: 0,
            rd_next: 0,
            is_alu_operation: false,
            is_alu_operation_next: false,
        }
    }

    pub fn get_memory_access_value_out(&self) -> MemoryAccessValue {
        MemoryAccessValue {
            alu_result: self.alu_result,
            rd: self.rd,
            is_alu_operation: self.is_alu_operation,
        }
    }
}

impl PipelineStage for InstructionMemoryAccess {
    fn compute(&mut self) {
        if (self.should_stall)() {
            return;
        }
        let execution_value = (self.get_execution_value_in)();
        self.alu_result_next = execution_value.alu_result;
        self.rd_next = execution_value.rd;
        self.is_alu_operation_next = execution_value.is_alu_operation;
    }

    fn latch_next(&mut self) {
        self.alu_result = self.alu_result_next;
        self.rd = self.rd_next;
        self.is_alu_operation = self.is_alu_operation_next;
    }
}
