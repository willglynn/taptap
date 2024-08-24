//! Gateway link layer escaping.

use bytes::{BufMut, BytesMut};

/// Determine the number of bytes needed to store the escaped version of a given input buffer.
pub fn escaped_length(input: &[u8]) -> usize {
    input.len()
        + input
            .iter()
            .filter(|b| matches!(**b, 0x7e | 0x23..=0x25 | 0xa3..=0xa5))
            .count()
}

#[derive(thiserror::Error, Debug, Copy, Clone, Eq, PartialEq)]
#[error("escaping error")]
pub struct InvalidEscapeSequence;

/// Apply link layer escaping.
pub fn escape(buffer: &[u8], output: &mut BytesMut) {
    output.reserve(buffer.len());

    for byte in buffer {
        let escaped = match byte {
            0x7e => 0x00,
            0x24 => 0x01,
            0x23 => 0x02,
            0x25 => 0x03,
            0xa4 => 0x04,
            0xa3 => 0x05,
            0xa5 => 0x06,
            _ => {
                output.put_u8(*byte);
                continue;
            }
        };
        output.put_u8(0x7e);
        output.put_u8(escaped);
    }
}

pub fn unescaped_byte(byte_after_0x7e: u8) -> Result<u8, InvalidEscapeSequence> {
    match byte_after_0x7e {
        0x00 => Ok(0x7e),
        0x01 => Ok(0x24),
        0x02 => Ok(0x23),
        0x03 => Ok(0x25),
        0x04 => Ok(0xa4),
        0x05 => Ok(0xa3),
        0x06 => Ok(0xa5),
        _ => Err(InvalidEscapeSequence),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLES: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"~", b"\x7e\x00"),
        (b"hello", b"hello"),
        (b"~hello~", b"\x7e\x00hello\x7e\x00"),
        (
            b"\x7e\xa3\xa4\xa5\x23\x24\x25abcdef",
            b"\x7e\x00\x7e\x05\x7e\x04\x7e\x06\x7e\x02\x7e\x01\x7e\x03abcdef",
        ),
        (
            &[
                0x92, 0x01, 0x01, 0x49, 0x00, 0xFF, 0x7C, 0xDB, 0xC2, 0xA3, 0x85,
            ],
            &[
                0x92, 0x01, 0x01, 0x49, 0x00, 0xFF, 0x7C, 0xDB, 0xC2, 0x7E, 0x05, 0x85,
            ],
        ),
    ];

    #[test]
    fn test_escaped_length() {
        for (raw, escaped) in EXAMPLES.iter().copied() {
            assert_eq!(escaped_length(raw), escaped.len(), "{:?}", raw);
        }
    }

    #[test]
    fn test_escape() {
        for (raw, escaped) in EXAMPLES.iter().copied() {
            let mut output = BytesMut::new();
            escape(raw, &mut output);
            assert_eq!(output, escaped, "{:?}", raw);
        }
    }
}
