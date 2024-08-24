use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
// Use `zerocopy` to transmute `#[repr(C)]` structs to/from byte slices
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

// Everything at this layer is big endian
use pv::application::PacketType;
use zerocopy::byteorder::big_endian::U16;

mod receiver;
use crate::gateway::link::{Address, GatewayID};
use crate::pv;
use crate::pv::link::SlotCounter;
pub use receiver::{Counters, Receiver, Sink};

#[derive(
    Debug,
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
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[repr(transparent)]
#[serde(transparent)]
pub struct CommandSequenceNumber(pub u8);

/// A command request frame payload.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable,
)]
#[repr(C)]
pub struct CommandRequest {
    pub unknown: [u8; 3],
    pub packet_type: PacketType,
    pub sequence_number: CommandSequenceNumber,
}

/// A command response frame payload.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable,
)]
#[repr(C)]
pub struct CommandResponse {
    pub unknown_1: u8,
    pub tx_buffers_free: u8,
    pub unknown_2: u8,
    pub packet_type: PacketType,
    pub command_sequence_number: CommandSequenceNumber,
}

/// A receive request frame payload.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable,
)]
#[repr(C)]
pub struct ReceiveRequest {
    pub unknown_1: [u8; 2],
    pub packet_number: U16,
    pub unknown_2: u8,
}

/// A receive response frame payload, decoded into its most general form.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ReceiveResponse {
    pub rx_buffers_used: Option<u8>,
    pub tx_buffers_free: Option<u8>,
    pub unknown_1: Option<[u8; 2]>,
    pub unknown_2: Option<[u8; 2]>,
    pub packet_number: u16,
    pub slot_counter: SlotCounter,
}

fn interpret_packet_number_lo(new_lo: u8, old: u16) -> u16 {
    let [old_hi, old_lo] = old.to_be_bytes();
    let new_hi = if new_lo >= old_lo {
        old_hi
    } else {
        // wrap
        old_hi + 1
    };
    u16::from_be_bytes([new_hi, new_lo])
}

#[derive(thiserror::Error, Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidReceiveResponse {
    #[error("too short: expected at least {0} bytes")]
    TooShort(usize),
    #[error("invalid status type: {0:#06x}")]
    UnknownStatusType(u16),
}

impl ReceiveResponse {
    /// Attempt to interpret a byte slice as a `ReceiveResponse`, using an existing packet number
    /// for reference.
    pub fn read_from_bytes(
        bytes: &[u8],
        packet_number: u16,
    ) -> Result<(Self, pv::network::ReceivedPackets), InvalidReceiveResponse> {
        // Ensure we have at least a minimal length
        if bytes.len() < 5 {
            return Err(InvalidReceiveResponse::TooShort(5));
        };

        // Read the status type bitmask
        let status_type = U16::ref_from_bytes(&bytes[0..2]).unwrap().get();

        // Ensure it matches the known patterns
        if status_type & 0xffe0 != 0x00e0 {
            return Err(InvalidReceiveResponse::UnknownStatusType(status_type));
        }

        // Split off the rest
        let (_, mut rest) = bytes.split_at(2);

        // Grab rx_buffers_used, if any
        let rx_buffers_used = if status_type & 0x0001 == 0 {
            if rest.is_empty() {
                return Err(InvalidReceiveResponse::TooShort(bytes.len() + 1));
            }
            let (value, new_rest) = rest.split_at(1);
            rest = new_rest;
            Some(value[0])
        } else {
            None
        };

        // Grab tx_buffers_free, if any
        let tx_buffers_free = if status_type & 0x0002 == 0 {
            if rest.is_empty() {
                return Err(InvalidReceiveResponse::TooShort(bytes.len() + 1));
            }
            let (value, new_rest) = rest.split_at(1);
            rest = new_rest;
            Some(value[0])
        } else {
            None
        };

        // Grab unknown_a, if any
        let unknown_a = if status_type & 0x0004 == 0 {
            if rest.is_empty() {
                return Err(InvalidReceiveResponse::TooShort(bytes.len() + 1));
            }
            let (value, new_rest) = rest.split_at(2);
            rest = new_rest;
            Some([value[0], value[1]])
        } else {
            None
        };

        // Grab unknown_b, if any
        let unknown_b = if status_type & 0x0008 == 0 {
            if rest.is_empty() {
                return Err(InvalidReceiveResponse::TooShort(bytes.len() + 1));
            }
            let (value, new_rest) = rest.split_at(2);
            rest = new_rest;
            Some([value[0], value[1]])
        } else {
            None
        };

        // Grab packet number, expanding as needed
        let packet_number = if status_type & 0x0010 == 0 {
            if rest.is_empty() {
                return Err(InvalidReceiveResponse::TooShort(bytes.len() + 1));
            }
            let (value, new_rest) = rest.split_at(2);
            rest = new_rest;
            u16::from_be_bytes([value[0], value[1]])
        } else {
            if rest.is_empty() {
                return Err(InvalidReceiveResponse::TooShort(bytes.len() + 1));
            }
            let (value, new_rest) = rest.split_at(1);
            rest = new_rest;
            interpret_packet_number_lo(value[0], packet_number)
        };

        // Grab slot counter
        if rest.len() < 2 {
            return Err(InvalidReceiveResponse::TooShort(bytes.len() + 2));
        }
        let (slot_counter, new_rest) = rest.split_at(2);
        rest = new_rest;
        let slot_counter = SlotCounter::read_from_bytes(slot_counter).unwrap();

        Ok((
            Self {
                rx_buffers_used,
                tx_buffers_free,
                unknown_1: unknown_a,
                unknown_2: unknown_b,
                packet_number,
                slot_counter,
            },
            pv::network::ReceivedPackets(rest),
        ))
    }
}

/// An identify response frame payload.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable,
)]
#[repr(C)]
pub struct IdentifyResponse {
    pub pv_long_address: pv::LongAddress,
    pub gateway_address: [u8; 2],
}

impl IdentifyResponse {
    pub fn gateway_id(&self) -> Option<GatewayID> {
        match Address::from(self.gateway_address) {
            Address::From(_) => None,
            Address::To(id) => Some(id),
        }
    }
}

/// An enumeration start request frame payload.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable,
)]
#[repr(C)]
pub struct EnumerationStartRequest {
    pub unknown: [u8; 4],
    pub enumeration_address: [u8; 2],
}

impl EnumerationStartRequest {
    pub fn enumeration_gateway_id(&self) -> Option<GatewayID> {
        match Address::from(self.enumeration_address) {
            Address::From(_) => None,
            Address::To(id) => Some(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pv::network::ReceivedPackets;

    #[test]
    fn rx_request_from_bytes() {
        assert_eq!(
            ReceiveRequest::read_from_bytes(&[0x00, 0x01, 0x18, 0x83, 0x04]),
            Ok(ReceiveRequest {
                unknown_1: [0x00, 0x01],
                packet_number: 0x1883.into(),
                unknown_2: 0x04,
            })
        );
    }

    #[test]
    fn rx_response_from_bytes() {
        assert_eq!(
            ReceiveResponse::read_from_bytes(
                &[0x00, 0xE0, 0x04, 0x0E, 0x00, 0x01, 0x02, 0x00, 0x40, 0xFB, 0x21, 0x1B, 1, 2, 3],
                0x40FB,
            ),
            Ok((
                ReceiveResponse {
                    rx_buffers_used: Some(0x04),
                    tx_buffers_free: Some(0x0E),
                    unknown_1: Some([0x00, 0x01]),
                    unknown_2: Some([0x02, 0x00]),
                    packet_number: 0x40FB,
                    slot_counter: 0x211B.into(),
                },
                ReceivedPackets(&[1, 2, 3])
            ))
        );

        assert_eq!(
            ReceiveResponse::read_from_bytes(&[0x00, 0xFE, 0x02, 0xFF, 0x21, 0x22, 4], 0x40FB),
            Ok((
                ReceiveResponse {
                    rx_buffers_used: Some(0x02),
                    tx_buffers_free: None,
                    unknown_1: None,
                    unknown_2: None,
                    packet_number: 0x40FF,
                    slot_counter: 0x2122.into(),
                },
                ReceivedPackets(&[4])
            ))
        );

        assert_eq!(
            ReceiveResponse::read_from_bytes(&[0x00, 0xEE, 0x00, 0x41, 0x01, 0x21, 0x27], 0x40FB),
            Ok((
                ReceiveResponse {
                    rx_buffers_used: Some(0x00),
                    tx_buffers_free: None,
                    unknown_1: None,
                    unknown_2: None,
                    packet_number: 0x4101,
                    slot_counter: 0x2127.into(),
                },
                ReceivedPackets(&[])
            ))
        );

        assert_eq!(
            ReceiveResponse::read_from_bytes(&[0x00, 0xFF, 0x03, 0x21, 0x31], 0x40FB),
            Ok((
                ReceiveResponse {
                    rx_buffers_used: None,
                    tx_buffers_free: None,
                    unknown_1: None,
                    unknown_2: None,
                    packet_number: 0x4103,
                    slot_counter: 0x2131.into(),
                },
                ReceivedPackets(&[])
            ))
        );
    }

    #[test]
    fn identify_response_payload() {
        let expected = IdentifyResponse {
            pv_long_address: pv::LongAddress([0x04, 0xC0, 0x5B, 0x30, 0x00, 0x02, 0xBE, 0x16]),
            gateway_address: [0x12, 0x01],
        };
        assert_eq!(
            IdentifyResponse::read_from_bytes(&[
                0x04, 0xC0, 0x5B, 0x30, 0x00, 0x02, 0xBE, 0x16, 0x12, 0x01
            ]),
            Ok(expected)
        );
        assert_eq!(expected.gateway_id(), Some(0x1201.try_into().unwrap()));
    }
}
