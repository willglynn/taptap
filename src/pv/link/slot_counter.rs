use std::convert::Into;
use super::*;

/// A slot counter.
///
/// Slot counters are logically divided into two components: an epoch and a slot number. Each epoch
/// takes 60 ± 1% seconds, so the slot counter repeats after 4 minutes.
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
pub struct SlotCounter(pub big_endian::U16);

impl SlotCounter {
    pub const ZERO: Self = SlotCounter(U16::ZERO);

    pub fn new(epoch: SlotEpoch, slot_number: SlotNumber) -> Self {
        let value: u16 = match epoch {
            SlotEpoch::Epoch0 => 0x0000,
            SlotEpoch::Epoch4 => 0x4000,
            SlotEpoch::Epoch8 => 0x8000,
            SlotEpoch::EpochC => 0xC000,
        } | slot_number.0;

        Self(value.into())
    }

    pub fn epoch(&self) -> SlotEpoch {
        match self.0.get() & 0xc000 {
            0x0000 => SlotEpoch::Epoch0,
            0x4000 => SlotEpoch::Epoch4,
            0x8000 => SlotEpoch::Epoch8,
            0xC000 => SlotEpoch::EpochC,
            _ => unreachable!(),
        }
    }

    pub fn slot_number(&self) -> Result<SlotNumber, InvalidSlotNumber> {
        (self.0.get() & 0x3fff).try_into()
    }

    pub fn slots_since(&self, past: &Self) -> Result<u16, InvalidSlotNumber> {
        let self_abs_slots = self.epoch() as u8 as u16 * 12000 + self.slot_number()?.0;
        let past_abs_slots = past.epoch() as u8 as u16 * 12000 + past.slot_number()?.0;

        Ok(if self_abs_slots > past_abs_slots {
            // Expected
            self_abs_slots - past_abs_slots
        } else {
            // We wrapped
            48000 - past_abs_slots + self_abs_slots
        })
    }
}

impl From<u16> for SlotCounter {
    fn from(value: u16) -> Self {
        Self(value.into())
    }
}
impl From<SlotCounter> for u16 {
    fn from(value: SlotCounter) -> Self {
        value.0.get()
    }
}

impl std::fmt::Debug for SlotCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.slot_number() {
            Ok(n) => f
                .debug_tuple("SlotCounter")
                .field(&self.epoch())
                .field(&n)
                .finish(),
            other => f
                .debug_tuple("SlotCounter")
                .field(&self.epoch())
                .field(&other)
                .finish(),
        }
    }
}

impl Serialize for SlotCounter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(self.0.get())
    }
}

impl<'de> Deserialize<'de> for SlotCounter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u16::deserialize(deserializer).map(U16::from).map(Self)
    }
}

/// An epoch of slot numbers, corresponding to approximately one minute of real time.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum SlotEpoch {
    Epoch0 = 0,
    Epoch4 = 1,
    Epoch8 = 2,
    EpochC = 3,
}

impl std::ops::Add<u8> for SlotEpoch {
    type Output = SlotEpoch;

    // SlotEpoch addition wraps
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: u8) -> Self::Output {
        match (self as u8).wrapping_add(rhs) % 4 {
            0 => Self::Epoch0,
            1 => Self::Epoch4,
            2 => Self::Epoch8,
            3 => Self::EpochC,
            _ => unreachable!()
        }
    }
}

impl std::ops::AddAssign<u8> for SlotEpoch {
    fn add_assign(&mut self, rhs: u8) {
        *self = *self + rhs;
    }
}

/// A slot number.
///
/// Each slot takes 5 ± 1% milliseconds. Slot numbers range from 0 to 11999 inclusive.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct SlotNumber(u16);

impl From<SlotNumber> for u16 {
    fn from(value: SlotNumber) -> Self {
        value.0
    }
}

impl TryFrom<u16> for SlotNumber {
    type Error = InvalidSlotNumber;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value <= 0x2edf {
            Ok(SlotNumber(value))
        } else {
            Err(InvalidSlotNumber(value))
        }
    }
}

#[derive(thiserror::Error, Debug, Copy, Clone, Eq, PartialEq)]
#[error("invalid slot number: {0:#06x}")]
pub struct InvalidSlotNumber(pub u16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let slot = SlotCounter(0x0000.into());
        assert_eq!(slot.epoch(), SlotEpoch::Epoch0);
        assert_eq!(slot.slot_number(), Ok(SlotNumber(0)));
        assert_eq!(SlotCounter::new(SlotEpoch::Epoch0, SlotNumber::try_from(0).unwrap()), slot);

        let slot = SlotCounter(0x2edf.into());
        assert_eq!(slot.epoch(), SlotEpoch::Epoch0);
        assert_eq!(slot.slot_number(), Ok(SlotNumber(11999)));
        assert_eq!(SlotCounter::new(SlotEpoch::Epoch0, SlotNumber::try_from(11999).unwrap()), slot);

        let slot = SlotCounter(0x2ee0.into());
        assert_eq!(slot.epoch(), SlotEpoch::Epoch0);
        assert_eq!(slot.slot_number(), Err(InvalidSlotNumber(0x2ee0)));
    }

    #[test]
    fn slots_since() {
        assert_eq!(SlotCounter(0x0000.into()).slots_since(&SlotCounter(0xeedf.into())), Ok(1));
        assert_eq!(SlotCounter(0x4000.into()).slots_since(&SlotCounter(0xeedf.into())), Ok(12001));
        assert_eq!(SlotCounter(0x8000.into()).slots_since(&SlotCounter(0xeedf.into())), Ok(24001));
        assert_eq!(SlotCounter(0xc000.into()).slots_since(&SlotCounter(0xeedf.into())), Ok(36001));

        assert_eq!(SlotCounter(0xeedf.into()).slots_since(&SlotCounter(0xc000.into())), Ok(11999));
        assert_eq!(SlotCounter(0xeedf.into()).slots_since(&SlotCounter(0x8000.into())), Ok(23999));
        assert_eq!(SlotCounter(0xeedf.into()).slots_since(&SlotCounter(0x4000.into())), Ok(35999));
        assert_eq!(SlotCounter(0xeedf.into()).slots_since(&SlotCounter(0x0000.into())), Ok(47999));

        assert_eq!(SlotCounter(0x6edf.into()).slots_since(&SlotCounter(0x4000.into())), Ok(11999));
        assert_eq!(SlotCounter(0x6edf.into()).slots_since(&SlotCounter(0x0000.into())), Ok(23999));
        assert_eq!(SlotCounter(0x6edf.into()).slots_since(&SlotCounter(0xc000.into())), Ok(35999));
        assert_eq!(SlotCounter(0x6edf.into()).slots_since(&SlotCounter(0x8000.into())), Ok(47999));

        assert_eq!(SlotCounter(0x0100.into()).slots_since(&SlotCounter(0x0080.into())), Ok(128));
        assert_eq!(SlotCounter(0x0100.into()).slots_since(&SlotCounter(0xc080.into())), Ok(12128));
        assert_eq!(SlotCounter(0x0100.into()).slots_since(&SlotCounter(0x8080.into())), Ok(24128));
        assert_eq!(SlotCounter(0x0100.into()).slots_since(&SlotCounter(0x4080.into())), Ok(36128));
    }
}
