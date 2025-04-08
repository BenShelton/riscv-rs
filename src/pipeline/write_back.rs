use crate::RegisterFile;

use super::{PipelineStage, memory_access::MemoryAccessValue};

pub struct InstructionWriteBack {
    should_stall: Box<dyn Fn() -> bool>,
    get_memory_access_value_in: Box<dyn Fn() -> MemoryAccessValue>,
    reg_file: RegisterFile,
}

pub struct InstructionWriteBackParams {
    pub should_stall: Box<dyn Fn() -> bool>,
    pub get_memory_access_value_in: Box<dyn Fn() -> MemoryAccessValue>,
    pub reg_file: RegisterFile,
}

impl InstructionWriteBack {
    pub fn new(params: InstructionWriteBackParams) -> Self {
        Self {
            should_stall: params.should_stall,
            get_memory_access_value_in: params.get_memory_access_value_in,
            reg_file: params.reg_file,
        }
    }
}

impl PipelineStage for InstructionWriteBack {
    fn compute(&mut self) {
        if !(self.should_stall)() {
            let memory_access_value = (self.get_memory_access_value_in)();
            if memory_access_value.is_alu_operation {
                self.reg_file.borrow_mut()[memory_access_value.rd as usize] =
                    memory_access_value.alu_result;
            }
        }
    }

    fn latch_next(&mut self) {}
}
