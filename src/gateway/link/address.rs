use schemars::JsonSchema;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::convert::TryFrom;

const DIRECTION_BIT: u16 = 0x8000;
const GATEWAY_ID_MASK: u16 = 0x7fff;

/// A gateway link layer address, which is either `To` or `From` a specific `GatewayID`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Address {
    /// `From` the indicated gateway, to the controller
    From(GatewayID),
    /// `To` the indicated gateway, from the controller
    To(GatewayID),
}

impl From<u16> for Address {
    fn from(value: u16) -> Self {
        let direction = value & DIRECTION_BIT;
        let id = GatewayID(value & GATEWAY_ID_MASK);

        if direction == 0 {
            Self::To(id)
        } else {
            Self::From(id)
        }
    }
}

impl From<Address> for u16 {
    fn from(value: Address) -> Self {
        match value {
            Address::From(id) => id.0 | DIRECTION_BIT,
            Address::To(id) => id.0,
        }
    }
}

impl From<[u8; 2]> for Address {
    fn from(value: [u8; 2]) -> Self {
        u16::from_be_bytes(value).into()
    }
}

impl From<Address> for [u8; 2] {
    fn from(value: Address) -> Self {
        u16::from(value).to_be_bytes()
    }
}

/// A 15-bit gateway ID.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct GatewayID(u16);

impl GatewayID {
    /// The all-zeroes gateway address
    pub const ZERO: GatewayID = GatewayID(0);
}

impl std::fmt::Debug for GatewayID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("GatewayID")
            .field(&format_args!("{:#04x}", self.0))
            .finish()
    }
}

impl std::fmt::Display for GatewayID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:#04x}", self.0)
    }
}

#[derive(thiserror::Error, Debug, Copy, Clone, Eq, PartialEq)]
#[error("invalid gateway ID {0:04x}")]
pub struct InvalidGatewayID(u16);

impl TryFrom<u16> for GatewayID {
    type Error = InvalidGatewayID;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value & GATEWAY_ID_MASK != value {
            Err(InvalidGatewayID(value))
        } else {
            Ok(GatewayID(value))
        }
    }
}

impl From<GatewayID> for u16 {
    fn from(value: GatewayID) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for GatewayID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let id = u16::deserialize(deserializer)?;
        Self::try_from(id).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_id() {
        assert_eq!(GatewayID::try_from(0), Ok(GatewayID(0)));
        assert_eq!(GatewayID::try_from(1), Ok(GatewayID(1)));
        assert_eq!(GatewayID::try_from(0x7fff), Ok(GatewayID(0x7fff)));
        assert_eq!(GatewayID::try_from(0x8000), Err(InvalidGatewayID(0x8000)));
        assert_eq!(GatewayID::try_from(0xffff), Err(InvalidGatewayID(0xffff)));

        assert_eq!(u16::from(GatewayID(1)), 1);

        assert_eq!(GatewayID(0x1201).to_string(), "0x1201");
    }

    #[test]
    fn address() {
        assert_eq!(Address::from([0x12, 0x01]), Address::To(GatewayID(0x1201)));
        assert_eq!(
            Address::from([0x92, 0x01]),
            Address::From(GatewayID(0x1201))
        );

        assert_eq!(
            [0x12, 0x01],
            <[u8; 2]>::from(Address::To(GatewayID(0x1201)))
        );
        assert_eq!(
            [0x92, 0x01],
            <[u8; 2]>::from(Address::From(GatewayID(0x1201)))
        );
    }

    #[test]
    fn address_fmt() {
        assert_eq!(format!("{:?}", &GatewayID(0x1201)), "GatewayID(0x1201)");
    }
}
