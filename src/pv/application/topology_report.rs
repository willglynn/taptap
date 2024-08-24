use super::*;
use crate::pv::network::NodeAddress;
use crate::pv::physical::RSSI;
use crate::pv::{LongAddress, ShortAddress};

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
)]
#[repr(C)]
pub struct TopologyReport {
    pub short_address: ShortAddress,
    pub pv_node_id: NodeAddress,
    pub next_hop: NodeAddress,
    pub unknown_1: [u8; 2],
    pub long_address: LongAddress,
    pub rssi: RSSI,
    pub unknown_2: [u8; 5],
}
