use crate::pv;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "String", into = "String")]
pub struct Barcode(pub pv::LongAddress);

const N2H: [u8; 16] = *b"0123456789ABCDEF";

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq)]
#[error("invalid barcode: {0:?}")]
pub struct InvalidBarcodeError(String);

impl std::str::FromStr for Barcode {
    type Err = InvalidBarcodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.get(1..2) != Some("-") || s.len() < 5 {
            return Err(InvalidBarcodeError(s.into()));
        }

        let leading_nibble =
            u8::from_str_radix(&s[0..1], 16).map_err(|_| InvalidBarcodeError(s.into()))?;
        let (middle, checksum) = s.split_at(s.len() - 1);
        let (_, rest) = middle.split_at(2);

        let rest = u64::from_str_radix(rest, 16).map_err(|_| InvalidBarcodeError(s.into()))?;

        let addr = rest | (0x04c05b0 | leading_nibble as u64) << 36;
        let addr = pv::LongAddress(addr.to_be_bytes());

        if [crc(addr)] == checksum.as_bytes() {
            Ok(Barcode(addr))
        } else {
            Err(InvalidBarcodeError(s.into()))
        }
    }
}

impl std::fmt::Display for Barcode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let bytes = &self.0 .0;

        // Barcode formatting only applies to addresses with a certain prefix
        if bytes[0] != 0x04 || bytes[1] != 0xc0 || bytes[2] != 0x5b {
            return write!(f, "{}", self.0);
        }

        f.write_char(N2H[(bytes[3] >> 4) as usize] as char)?;
        f.write_char('-')?;

        let nibbles = [
            bytes[3] & 0xf,
            bytes[4] >> 4,
            bytes[4] & 0xf,
            bytes[5] >> 4,
            bytes[5] & 0xf,
            bytes[6] >> 4,
            bytes[6] & 0xf,
            bytes[7] >> 4,
            bytes[7] & 0xf,
        ];

        let mut skipping = true;
        for (i, nibble) in nibbles.iter().copied().enumerate() {
            match (skipping, nibble) {
                (true, 0) if i < 10 => {
                    // Let it roll
                    continue;
                }
                (true, _) => {
                    // Stop skipping
                    skipping = false;
                }
                _ => {}
            }
            f.write_char(N2H[nibble as usize] as char)?;
        }

        f.write_char(crc(self.0) as char)
    }
}

impl TryFrom<String> for Barcode {
    type Error = InvalidBarcodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Barcode> for String {
    fn from(value: Barcode) -> Self {
        value.to_string()
    }
}

// https://stackoverflow.com/q/54507106/1026671
// "it seems unlikely that anyone would use a CRC with so few bytes in a practical application"
const TABLE: &[u8] = &[
    0x0, 0x3, 0x6, 0x5, 0xc, 0xf, 0xa, 0x9, 0xb, 0x8, 0xd, 0xe, 0x7, 0x4, 0x1, 0x2, 0x5, 0x6, 0x3,
    0x0, 0x9, 0xa, 0xf, 0xc, 0xe, 0xd, 0x8, 0xb, 0x2, 0x1, 0x4, 0x7, 0xa, 0x9, 0xc, 0xf, 0x6, 0x5,
    0x0, 0x3, 0x1, 0x2, 0x7, 0x4, 0xd, 0xe, 0xb, 0x8, 0xf, 0xc, 0x9, 0xa, 0x3, 0x0, 0x5, 0x6, 0x4,
    0x7, 0x2, 0x1, 0x8, 0xb, 0xe, 0xd, 0x7, 0x4, 0x1, 0x2, 0xb, 0x8, 0xd, 0xe, 0xc, 0xf, 0xa, 0x9,
    0x0, 0x3, 0x6, 0x5, 0x2, 0x1, 0x4, 0x7, 0xe, 0xd, 0x8, 0xb, 0x9, 0xa, 0xf, 0xc, 0x5, 0x6, 0x3,
    0x0, 0xd, 0xe, 0xb, 0x8, 0x1, 0x2, 0x7, 0x4, 0x6, 0x5, 0x0, 0x3, 0xa, 0x9, 0xc, 0xf, 0x8, 0xb,
    0xe, 0xd, 0x4, 0x7, 0x2, 0x1, 0x3, 0x0, 0x5, 0x6, 0xf, 0xc, 0x9, 0xa, 0xe, 0xd, 0x8, 0xb, 0x2,
    0x1, 0x4, 0x7, 0x5, 0x6, 0x3, 0x0, 0x9, 0xa, 0xf, 0xc, 0xb, 0x8, 0xd, 0xe, 0x7, 0x4, 0x1, 0x2,
    0x0, 0x3, 0x6, 0x5, 0xc, 0xf, 0xa, 0x9, 0x4, 0x7, 0x2, 0x1, 0x8, 0xb, 0xe, 0xd, 0xf, 0xc, 0x9,
    0xa, 0x3, 0x0, 0x5, 0x6, 0x1, 0x2, 0x7, 0x4, 0xd, 0xe, 0xb, 0x8, 0xa, 0x9, 0xc, 0xf, 0x6, 0x5,
    0x0, 0x3, 0x9, 0xa, 0xf, 0xc, 0x5, 0x6, 0x3, 0x0, 0x2, 0x1, 0x4, 0x7, 0xe, 0xd, 0x8, 0xb, 0xc,
    0xf, 0xa, 0x9, 0x0, 0x3, 0x6, 0x5, 0x7, 0x4, 0x1, 0x2, 0xb, 0x8, 0xd, 0xe, 0x3, 0x0, 0x5, 0x6,
    0xf, 0xc, 0x9, 0xa, 0x8, 0xb, 0xe, 0xd, 0x4, 0x7, 0x2, 0x1, 0x6, 0x5, 0x0, 0x3, 0xa, 0x9, 0xc,
    0xf, 0xd, 0xe, 0xb, 0x8, 0x1, 0x2, 0x7, 0x4,
];

const C2H: [u8; 16] = *b"GHJKLMNPRSTVWXYZ";

fn crc(addr: pv::LongAddress) -> u8 {
    let mut crc = 2;
    for byte in addr.0.as_slice() {
        crc = TABLE[(*byte ^ (crc << 4)) as usize];
    }

    C2H[crc as usize]
}

impl From<&Barcode> for pv::LongAddress {
    fn from(value: &Barcode) -> Self {
        value.0
    }
}
impl From<Barcode> for pv::LongAddress {
    fn from(value: Barcode) -> Self {
        value.0
    }
}
impl From<pv::LongAddress> for Barcode {
    fn from(value: pv::LongAddress) -> Self {
        Self(value)
    }
}
impl From<&pv::LongAddress> for Barcode {
    fn from(value: &pv::LongAddress) -> Self {
        Self(*value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const ADDR: pv::LongAddress = pv::LongAddress([0x04, 0xC0, 0x5B, 0x40, 0x00, 0x9A, 0x57, 0xA2]);
    const BARCODE: &str = "4-9A57A2L";

    #[test]
    fn crc() {
        assert_eq!(
            super::crc(pv::LongAddress([
                0x04, 0xC0, 0x5B, 0x40, 0x00, 0x9A, 0x57, 0xA2
            ])),
            'L' as u8
        );
        assert_eq!(
            super::crc(pv::LongAddress([
                0x04, 0xC0, 0x5B, 0x40, 0x00, 0x79, 0xAC, 0x16
            ])),
            'V' as u8
        );
        assert_eq!(
            super::crc(pv::LongAddress([
                0x04, 0xC0, 0x5B, 0x40, 0x00, 0x79, 0xAB, 0x99
            ])),
            'W' as u8
        );
    }

    #[test]
    fn display() {
        assert_eq!(Barcode(ADDR).to_string(), BARCODE);
    }

    #[test]
    fn parse() {
        assert_eq!(Barcode::from_str("4-9A57A2L"), Ok(Barcode(ADDR)));
        assert!(Barcode::from_str("4-9A57A2G").is_err());
    }

    #[test]
    fn long_address_conversion() {
        assert_eq!(pv::LongAddress::from(Barcode(ADDR)), ADDR);
        assert_eq!(Barcode::from(ADDR), Barcode(ADDR));
    }
}
