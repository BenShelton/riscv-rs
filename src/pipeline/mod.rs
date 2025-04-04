pub mod decode;
pub mod execute;
pub mod fetch;

pub trait PipelineStage {
    fn compute(&mut self);
    fn latch_next(&mut self);
}
