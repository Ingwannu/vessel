//! Invalid or non-UTF8 percent encodings remain literal, as in the reference.

const HEX_RADIX: u32 = 16;

pub(super) fn decode(value: &str) -> Option<String> {
    let mut bytes = value.bytes();
    let mut out = Vec::with_capacity(value.len());
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = char::from(bytes.next()?).to_digit(HEX_RADIX)?;
            let low = char::from(bytes.next()?).to_digit(HEX_RADIX)?;
            let decoded = high.checked_mul(HEX_RADIX)?.checked_add(low)?;
            out.push(u8::try_from(decoded).ok()?);
        } else {
            out.push(byte);
        }
    }
    String::from_utf8(out).ok()
}
