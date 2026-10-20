/// Lowercase hex, two digits per byte.
///
/// digest 0.11 returns a `hybrid-array`, which does not implement `LowerHex`,
/// so `format!("{:x}", hasher.finalize())` no longer compiles. This is the
/// same rendering `{:02x}` produced on the old `GenericArray`.
pub(crate) fn encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(char::from(DIGITS[(byte >> 4) as usize]));
        out.push(char::from(DIGITS[(byte & 0x0f) as usize]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty() {
        assert_eq!(encode(&[]), "");
    }

    #[test]
    fn encode_keeps_leading_zero_nibbles() {
        assert_eq!(encode(&[0x00, 0x0f, 0xab, 0xcd]), "000fabcd");
    }
}
