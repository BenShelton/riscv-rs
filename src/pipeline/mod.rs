mod instruction_fetch;

pub trait PipelineStage {
    fn ready_to_send(&self) -> bool;
    fn ready_to_receive(&self) -> bool;
    fn compute(&mut self);
    fn latch_next(&mut self);
}
