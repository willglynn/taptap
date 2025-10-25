use super::*;
use crate::pv::network::NodeAddress;
use crate::pv::LongAddress;
use zerocopy::big_endian;

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
)]
#[repr(C)]
pub struct NodeTableRequest {
    pub start_at: NodeAddress,
}

#[derive(Debug, FromBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
pub struct NodeTableResponse {
    pub start_at: NodeAddress,
    pub entries_count: big_endian::U16,
    pub entries: [NodeTableResponseEntry],
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
)]
#[repr(C)]
pub struct NodeTableResponseEntry {
    pub long_address: LongAddress,
    pub node_id: NodeAddress,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request() {
        assert_eq!(
            NodeTableRequest::ref_from_bytes(b"\x00\x02"),
            Ok(&NodeTableRequest { start_at: 2.into() })
        );
    }

    #[test]
    fn response() {
        let response = NodeTableResponse::ref_from_bytes(b"\x00\x00").unwrap();
        assert_eq!(response.entries_count.get(), 0);
        assert_eq!(response.entries.len(), 0);

        let response = NodeTableResponse::ref_from_bytes(
            b"\x00\x0C\x04\xC0\x5B\x40\x00\xA2\x34\x6F\x00\x02\x04\xC0\x5B\x40\x00\xA2\x34\x71\x00\x03",
        ).unwrap();
        assert_eq!(response.entries_count.get(), 0x000c);
        assert_eq!(response.entries.len(), 2);
        assert_eq!(
            response.entries[0].long_address,
            LongAddress([0x04, 0xC0, 0x5B, 0x40, 0x00, 0xA2, 0x34, 0x6F])
        );
        assert_eq!(response.entries[0].node_id, 0x0002.into());
        assert_eq!(
            response.entries[1].long_address,
            LongAddress([0x04, 0xC0, 0x5B, 0x40, 0x00, 0xA2, 0x34, 0x71])
        );
        assert_eq!(response.entries[1].node_id, 0x0003.into());
    }
}
