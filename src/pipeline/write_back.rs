use crate::RegisterFile;

use super::{PipelineStage, memory_access::MemoryAccessValue};

pub struct InstructionWriteBack {}

pub struct InstructionWriteBackParams<'a> {
    pub should_stall: bool,
    pub memory_access_value_in: MemoryAccessValue,
    pub reg_file: &'a mut RegisterFile,
}

impl InstructionWriteBack {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'a> PipelineStage<InstructionWriteBackParams<'a>> for InstructionWriteBack {
    fn compute(&mut self, params: InstructionWriteBackParams<'a>) {
        if params.should_stall {
            return;
        }
        let memory_access_value = params.memory_access_value_in;
        if memory_access_value.is_alu_operation {
            params.reg_file[memory_access_value.rd as usize] = memory_access_value.alu_result;
        }
    }

    fn latch_next(&mut self) {}
}
