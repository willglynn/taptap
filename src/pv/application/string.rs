use super::*;
use std::convert::TryFrom;

#[derive(
    Debug, Eq, PartialEq
)]
#[repr(C)]
pub struct StringRequest {
    pub pv_node_id: pv::network::NodeID,
    pub request: [u8],
}
impl Message for StringRequest {
    const PACKET_TYPE: PacketType = PacketType::STRING_REQUEST;
}

impl TryFrom<&StringRequest> for &str {
    type Error = std::str::Utf8Error;

    fn try_from(value: &StringRequest) -> Result<Self, Self::Error> {
        std::str::from_utf8(&value.request)
    }
}

impl From<&StringRequest> for String {
    fn from(value: &StringRequest) -> Self {
        String::from_utf8_lossy(&value.request).into()
    }
}
impl std::fmt::Display for StringRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.request))
    }
}

#[derive(
    Debug, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable,
)]
#[repr(C)]
pub struct StringResponse {
    pub response: [u8],
}
impl Message for StringResponse {
    const PACKET_TYPE: PacketType = PacketType::STRING_RESPONSE;
}

impl TryFrom<&StringResponse> for &str {
    type Error = std::str::Utf8Error;

    fn try_from(value: &StringResponse) -> Result<Self, Self::Error> {
        std::str::from_utf8(&value.response)
    }
}

impl From<&StringResponse> for String {
    fn from(value: &StringRequest) -> Self {
        String::from_utf8_lossy(&value.request).into()
    }
}
impl std::fmt::Display for StringResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.response))
    }
}
