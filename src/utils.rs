pub fn sign_extend_32(bits: u32, value: i32) -> i32 {
    let extend_bits = 32 - bits;
    (value << extend_bits) >> extend_bits
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
}
