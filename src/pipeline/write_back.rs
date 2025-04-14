use crate::RegisterFile;

use super::{PipelineStage, decode::DecodedInstruction, memory_access::MemoryAccessValue};

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
        match memory_access_value.instruction {
            DecodedInstruction::Alu { rd, .. } => {
                params.reg_file[rd as usize] = memory_access_value.write_back_value;
            }
            DecodedInstruction::Store { .. } => {
                // Store operations do not write back to the register file
            }
            DecodedInstruction::Load { rd, .. } => {
                params.reg_file[rd as usize] = memory_access_value.write_back_value;
            }
            DecodedInstruction::None => {}
        }
    }

    fn latch_next(&mut self) {}
}
