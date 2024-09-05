use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use std::num::{NonZeroU16, TryFromIntError};
use zerocopy::{big_endian, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// A 16-bit PV network layer node ID.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[repr(transparent)]
pub struct NodeID(NonZeroU16);
impl NodeID {
    pub const GATEWAY: Self = NodeID(NonZeroU16::MIN);
    pub const MAX: Self = NodeID(NonZeroU16::MAX);

    pub fn successor(&self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

impl std::fmt::Debug for NodeID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("NodeID")
            .field(&format_args!("{:#06X}", u16::from(self.0)))
            .finish()
    }
}

impl std::fmt::Display for NodeID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:#06X}", u16::from(self.0))
    }
}
impl TryFrom<u16> for NodeID {
    type Error = TryFromIntError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        NonZeroU16::try_from(value).map(Self)
    }
}

/// A 16-bit PV network layer node address, which could be either a `NodeID` or the broadcast
/// address.
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
pub struct NodeAddress(pub big_endian::U16);
impl NodeAddress {
    pub const ZERO: Self = Self(big_endian::U16::ZERO);
    pub const GATEWAY: Self = Self(big_endian::U16::new(1));
}

impl std::fmt::Debug for NodeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if *self == Self::ZERO {
            f.write_str("NodeAddress::ZERO")
        } else if *self == Self::GATEWAY {
            f.write_str("NodeAddress::GATEWAY")
        } else {
            f.debug_tuple("NodeAddress")
                .field(&format_args!("{:#06X}", u16::from(self.0)))
                .finish()
        }
    }
}

impl From<NodeAddress> for Option<NodeID> {
    fn from(value: NodeAddress) -> Self {
        match value {
            NodeAddress::ZERO => None,
            NodeAddress(id) => {
                let id = u16::from(id);
                assert_ne!(id, 0); // would be BROADCAST
                Some(NodeID(id.try_into().unwrap()))
            }
        }
    }
}

impl From<Option<NodeID>> for NodeAddress {
    fn from(value: Option<NodeID>) -> Self {
        match value {
            None => NodeAddress::ZERO,
            Some(NodeID(id)) => NodeAddress(u16::from(id).into()),
        }
    }
}
impl From<NodeID> for NodeAddress {
    fn from(value: NodeID) -> Self {
        NodeAddress(u16::from(value.0).into())
    }
}
impl TryFrom<NodeAddress> for NodeID {
    type Error = TryFromIntError;

    fn try_from(value: NodeAddress) -> Result<Self, Self::Error> {
        NonZeroU16::try_from(value.0.get()).map(Self)
    }
}
impl From<u16> for NodeAddress {
    fn from(value: u16) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for NodeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:04X}", u16::from(self.0))
    }
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable,
)]
#[repr(C)]
pub struct ReceivedPacketHeader {
    pub packet_type: application::PacketType,
    pub node_address: NodeAddress,
    pub short_address: ShortAddress,
    pub dsn: link::DSN,
    pub data_length: u8,
}

#[derive(thiserror::Error, Debug)]
#[error("packet too short")]
pub struct PacketTooShortError;

/// An `Iterator` over zero or more received packets.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReceivedPackets<'a>(pub &'a [u8]);
impl<'a> Iterator for ReceivedPackets<'a> {
    type Item = Result<(&'a ReceivedPacketHeader, &'a [u8]), PacketTooShortError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        } else if self.0.len() < size_of::<ReceivedPacketHeader>() {
            self.0 = &[];
            return Some(Err(PacketTooShortError));
        }

        let (header, rest) = self.0.split_at(size_of::<ReceivedPacketHeader>());
        let header = ReceivedPacketHeader::ref_from_bytes(header).unwrap(); // infallible

        let data_length = header.data_length as usize;
        if rest.len() < data_length {
            self.0 = &[];
            return Some(Err(PacketTooShortError));
        }
        let (data, rest) = rest.split_at(data_length);
        self.0 = rest;

        Some(Ok((header, data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id() {
        assert_eq!(NodeID::GATEWAY, NodeID::try_from(1).unwrap());
        assert_eq!(NodeID::MAX, NodeID::try_from(65535).unwrap());

        assert_eq!(
            NodeID::GATEWAY.successor(),
            Some(NodeID(NonZeroU16::try_from(2).unwrap()))
        );
        assert_eq!(NodeID::MAX.successor(), None);

        assert_eq!(NodeID::try_from(NodeAddress::GATEWAY), Ok(NodeID::GATEWAY));
        assert!(NodeID::try_from(NodeAddress(0.into())).is_err());

        assert_eq!(format!("{:?}", &NodeID::GATEWAY), "NodeID(0x0001)");
        assert_eq!(format!("{}", &NodeID::GATEWAY), "0x0001");
    }
}
