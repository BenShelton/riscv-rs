pub mod decode;
pub mod execute;
pub mod fetch;
pub mod memory_access;
pub mod write_back;

pub trait PipelineStage<T> {
    fn compute(&mut self, params: T);
    fn latch_next(&mut self);
}
