use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zerocopy::{big_endian, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, U16};

mod slot_counter;
pub use slot_counter::{InvalidSlotNumber, SlotCounter, SlotEpoch, SlotNumber};

/// A 16-bit PV link layer (802.15.4) short address.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    FromBytes,
    IntoBytes,
    Unaligned,
    KnownLayout,
    Immutable,
)]
#[repr(transparent)]
pub struct ShortAddress(pub big_endian::U16);

impl std::fmt::Debug for ShortAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("ShortAddress")
            .field(&format_args!("{:#06X}", u16::from(self.0)))
            .finish()
    }
}

impl std::fmt::Display for ShortAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:#06X}", u16::from(self.0))
    }
}

/// A 64-bit PV link layer (802.15.4) long address.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
    JsonSchema,
    FromBytes,
    IntoBytes,
    Unaligned,
    KnownLayout,
    Immutable,
)]
#[repr(transparent)]
pub struct LongAddress(pub [u8; 8]);

impl LongAddress {
    pub fn barcode(&self) -> crate::barcode::Barcode {
        self.into()
    }
}

impl std::fmt::Debug for LongAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("LongAddress")
            .field(&format_args!(
                "[{:#04X},{:#04X},{:#04X},{:#04X},{:#04X},{:#04X},{:#04X},{:#04X}]",
                self.0[0],
                self.0[1],
                self.0[2],
                self.0[3],
                self.0[4],
                self.0[5],
                self.0[6],
                self.0[7],
            ))
            .finish()
    }
}

impl std::fmt::Display for LongAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7],
        )
    }
}

#[derive(Copy, Clone, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable)]
#[repr(transparent)]
pub struct DSN(pub u8);

impl std::ops::Add<u8> for DSN {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl std::fmt::Debug for DSN {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("DSN")
            .field(&format_args!("{:#04X}", self.0))
            .finish()
    }
}

impl std::fmt::Display for DSN {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:#04X}", self.0)
    }
}
