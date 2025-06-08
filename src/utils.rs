#[derive(Default, Debug, PartialEq, Eq)]
pub struct LatchValue<T>
where
    T: Clone,
{
    value: T,
    next: T,
    default: T,
}

impl<T> LatchValue<T>
where
    T: Clone,
{
    pub fn new(value: T) -> Self {
        LatchValue {
            value: value.clone(),
            next: value.clone(),
            default: value,
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

    pub fn reset(&mut self) {
        self.value = self.default.clone();
        self.next = self.default.clone();
    }
}

pub fn sign_extend_32(bits: u32, value: i32) -> i32 {
    let extend_bits = 32 - bits;
    (value << extend_bits) >> extend_bits
}

pub fn slice_32(from: u32, to: u32, value: u32, position: u32) -> u32 {
    let span = from - to + 1;
    let sliced = (value >> to) & ((1 << span) - 1);

    if position != 0 {
        return sliced << (position - span);
    }

    sliced
}

pub fn bit(index: u32, value: u32, position: u32) -> u32 {
    ((value >> index) & 1) << (position - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_extend_32() {
        assert_eq!(sign_extend_32(8, 0xFF), -1);
        assert_eq!(sign_extend_32(8, 0x7F), 127);
        assert_eq!(sign_extend_32(16, 0xFFFF), -1);
        assert_eq!(sign_extend_32(16, 0x7FFF), 32767);
        assert_eq!(sign_extend_32(32, 0x7FFFFFFF), 2147483647);
    }

    #[test]
    fn test_slice_32() {
        assert_eq!(
            slice_32(31, 24, 0b1111_1111_0000_0000_0000_0000_0000_0000, 0),
            0b1111_1111
        );
        assert_eq!(
            slice_32(23, 16, 0b0000_0000_1111_1111_0000_0000_0000_0000, 0),
            0b1111_1111
        );
        assert_eq!(
            slice_32(15, 8, 0b0000_0000_0000_0000_1010_1010_0000_0000, 20),
            0b1010_1010_0000_0000_0000
        );
    }

    #[test]
    fn test_bit() {
        assert_eq!(bit(0, 0b0000_0001, 1), 0b1);
        assert_eq!(bit(1, 0b0000_0010, 2), 0b10);
        assert_eq!(bit(2, 0b0000_0100, 3), 0b100);
        assert_eq!(bit(3, 0b0000_1000, 4), 0b1000);
        assert_eq!(bit(4, 0b0001_0000, 5), 0b1_0000);
    }
}
