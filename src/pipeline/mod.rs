pub mod decode;
pub mod execute;
pub mod fetch;
pub mod memory_access;
pub mod write_back;

pub trait PipelineStage {
    fn compute(&mut self);
    fn latch_next(&mut self);
}

pub struct LatchValue<T>
where
    T: Clone,
{
    value: T,
    next: T,
}

impl<T> LatchValue<T>
where
    T: Clone,
{
    pub fn new(value: T) -> Self {
        LatchValue {
            value: value.clone(),
            next: value,
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn set(&mut self, value: T) {
        self.next = value;
    }

    pub fn latch_next(&mut self) {
        self.value = self.next.clone();
    }
}
