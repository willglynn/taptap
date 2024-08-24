use super::*;

#[derive(Copy, Clone, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable)]
#[repr(transparent)]
pub struct PacketType(pub u8);
impl PacketType {
    pub const STRING_REQUEST: Self = Self(0x06);
    pub const STRING_RESPONSE: Self = Self(0x07);
    pub const TOPOLOGY_REPORT: Self = Self(0x09);
    pub const GATEWAY_RADIO_CONFIGURATION_REQUEST: Self = Self(0x0D);
    pub const GATEWAY_RADIO_CONFIGURATION_RESPONSE: Self = Self(0x0E);
    pub const PV_CONFIGURATION_REQUEST: Self = Self(0x13);
    pub const PV_CONFIGURATION_RESPONSE: Self = Self(0x18);
    pub const BROADCAST: Self = Self(0x22);
    pub const BROADCAST_ACK: Self = Self(0x23);
    pub const NODE_TABLE_REQUEST: Self = Self(0x26);
    pub const NODE_TABLE_RESPONSE: Self = Self(0x27);
    pub const LONG_NETWORK_STATUS_REQUEST: Self = Self(0x2D);
    pub const NETWORK_STATUS_REQUEST: Self = Self(0x2E);
    pub const NETWORK_STATUS_RESPONSE: Self = Self(0x2F);
    pub const POWER_REPORT: Self = Self(0x31);
}

impl std::fmt::Debug for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            PacketType::STRING_REQUEST => f.write_str("PacketType::STRING_REQUEST"),
            PacketType::STRING_RESPONSE => f.write_str("PacketType::STRING_RESPONSE"),
            PacketType::TOPOLOGY_REPORT => f.write_str("PacketType::TOPOLOGY_REPORT"),
            PacketType::GATEWAY_RADIO_CONFIGURATION_REQUEST => {
                f.write_str("PacketType::GATEWAY_RADIO_CONFIGURATION_REQUEST")
            }
            PacketType::GATEWAY_RADIO_CONFIGURATION_RESPONSE => {
                f.write_str("PacketType::GATEWAY_RADIO_CONFIGURATION_RESPONSE")
            }
            PacketType::PV_CONFIGURATION_REQUEST => {
                f.write_str("PacketType::PV_CONFIGURATION_REQUEST")
            }
            PacketType::PV_CONFIGURATION_RESPONSE => {
                f.write_str("PacketType::PV_CONFIGURATION_RESPONSE")
            }
            PacketType::BROADCAST => f.write_str("PacketType::BROADCAST"),
            PacketType::BROADCAST_ACK => f.write_str("PacketType::BROADCAST_ACK"),
            PacketType::NODE_TABLE_REQUEST => f.write_str("PacketType::NODE_TABLE_REQUEST"),
            PacketType::NODE_TABLE_RESPONSE => f.write_str("PacketType::NODE_TABLE_RESPONSE"),
            PacketType::LONG_NETWORK_STATUS_REQUEST => {
                f.write_str("PacketType::LONG_NETWORK_STATUS_REQUEST")
            }
            PacketType::NETWORK_STATUS_REQUEST => f.write_str("PacketType::NETWORK_STATUS_REQUEST"),
            PacketType::NETWORK_STATUS_RESPONSE => {
                f.write_str("PacketType::NETWORK_STATUS_RESPONSE")
            }
            PacketType::POWER_REPORT => f.write_str("PacketType::POWER_REPORT"),
            _ => f
                .debug_tuple("PacketType")
                .field(&format_args!("{:#04X}", self.0))
                .finish(),
        }
    }
}

impl std::fmt::Display for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            PacketType::STRING_REQUEST => f.write_str("STRING_REQUEST"),
            PacketType::STRING_RESPONSE => f.write_str("STRING_RESPONSE"),
            PacketType::TOPOLOGY_REPORT => f.write_str("TOPOLOGY_REPORT"),
            PacketType::GATEWAY_RADIO_CONFIGURATION_REQUEST => {
                f.write_str("GATEWAY_RADIO_CONFIGURATION_REQUEST")
            }
            PacketType::GATEWAY_RADIO_CONFIGURATION_RESPONSE => {
                f.write_str("GATEWAY_RADIO_CONFIGURATION_RESPONSE")
            }
            PacketType::PV_CONFIGURATION_REQUEST => f.write_str("PV_CONFIGURATION_REQUEST"),
            PacketType::PV_CONFIGURATION_RESPONSE => f.write_str("PV_CONFIGURATION_RESPONSE"),
            PacketType::BROADCAST => f.write_str("BROADCAST"),
            PacketType::BROADCAST_ACK => f.write_str("BROADCAST_ACK"),
            PacketType::NODE_TABLE_REQUEST => f.write_str("NODE_TABLE_REQUEST"),
            PacketType::NODE_TABLE_RESPONSE => f.write_str("NODE_TABLE_RESPONSE"),
            PacketType::LONG_NETWORK_STATUS_REQUEST => f.write_str("LONG_NETWORK_STATUS_REQUEST"),
            PacketType::NETWORK_STATUS_REQUEST => f.write_str("NETWORK_STATUS_REQUEST"),
            PacketType::NETWORK_STATUS_RESPONSE => f.write_str("NETWORK_STATUS_RESPONSE"),
            PacketType::POWER_REPORT => f.write_str("POWER_REPORT"),
            _ => write!(f, "{:#04X}", self.0),
        }
    }
}
