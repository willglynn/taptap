use super::*;
use crate::pv::physical::RSSI;
use crate::pv::SlotCounter;

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
)]
#[repr(C)]
pub struct PowerReport {
    pub voltage_in_and_voltage_out: U12Pair,
    pub dc_dc_duty_cycle: u8,
    pub current_and_temperature: U12Pair,
    pub unknown: [u8; 3],
    pub slot_counter: SlotCounter,
    pub rssi: RSSI,
}

/// A pair of 12-bit unsigned integers packed into a single `[u8; 3]`.
#[derive(
    Copy, Clone, Eq, PartialEq, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
)]
#[repr(C)]
pub struct U12Pair(pub [u8; 3]);

impl std::fmt::Debug for U12Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let (a, b): (u16, u16) = (*self).into();
        f.debug_tuple("U12Pair")
            .field(&format_args!("{:#05x}", a))
            .field(&format_args!("{:#05x}", b))
            .finish()
    }
}

impl From<U12Pair> for (u16, u16) {
    fn from(value: U12Pair) -> Self {
        (
            u16::from_be_bytes([value.0[0], value.0[1]]) >> 4,
            u16::from_be_bytes([value.0[1], value.0[2]]) & 0x0fff,
        )
    }
}
impl TryFrom<(u16, u16)> for U12Pair {
    type Error = ();

    fn try_from((a, b): (u16, u16)) -> Result<Self, Self::Error> {
        if a & 0xfff != a || b & 0xfff != b {
            Err(())
        } else {
            let a = (a << 4).to_be_bytes();
            let b = b.to_be_bytes();
            Ok(Self([a[0], a[1] | b[0], b[1]]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u12_pair() {
        let pair = U12Pair([0x2b, 0x61, 0x58]);
        assert_eq!(<(u16, u16)>::from(pair), (0x2b6, 0x158));
        assert_eq!(<U12Pair>::try_from((0x2b6, 0x158)), Ok(pair));
    }
}
