pub mod decode;
pub mod execute;
pub mod fetch;
pub mod memory_access;
pub mod write_back;

pub trait PipelineStage {
    fn compute(&mut self);
    fn latch_next(&mut self);
}
