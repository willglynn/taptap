//! The gateway link layer.

mod address;

pub use address::{Address, GatewayID, InvalidGatewayID};

mod crc;

mod escaping;
mod receive;
pub use receive::{Counters, Receiver, Sink};

/// A gateway link layer frame.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Frame {
    pub address: Address,
    pub frame_type: Type,
    pub payload: Vec<u8>,
}

impl Frame {
    /// Encode the frame into `Bytes` ready for transmission by the physical layer, including a
    /// preamble.
    pub fn encode(&self) -> Vec<u8> {
        let start = match self.address {
            Address::From(_) => [0xff, 0x7e, 0x07].as_slice(),
            Address::To(_) => [0x00, 0xff, 0xff, 0x7e, 0x07].as_slice(),
        };
        let end = &[0x7e, 0x08];

        let mut output_buffer = Vec::with_capacity(
            start.len()
                + 4 // worst case escaped address
                + 4 // worst case escaped frame type
                + escaping::escaped_length(&self.payload)
                + 4 // worst case CRC
                + end.len(), // frame end
        );
        let initial_output_buffer_capacity = output_buffer.capacity();

        // Add the start sequence
        output_buffer.extend_from_slice(start);

        // Assemble the middle
        let mut body = Vec::with_capacity(2 + 2 + self.payload.len() + 2);
        let initial_body_capacity = body.capacity();
        body.extend_from_slice(&<[u8; 2]>::from(self.address));
        body.extend_from_slice(&self.frame_type.0.to_be_bytes());
        body.extend_from_slice(&self.payload);

        // Calculate and append the CRC
        let crc = crc::crc(&body);
        body.extend_from_slice(&crc.to_le_bytes());

        // Append the escaped content to the output buffer
        escaping::escape(&body, &mut output_buffer);

        // Append the terminator
        output_buffer.extend_from_slice(end);

        // Ensure we didn't need to reallocate
        debug_assert_eq!(body.capacity(), initial_body_capacity);
        debug_assert_eq!(output_buffer.capacity(), initial_output_buffer_capacity);

        // Ensure we didn't over-allocate
        debug_assert_eq!(body.len(), initial_body_capacity);
        debug_assert!(initial_output_buffer_capacity <= output_buffer.len() + 6);

        output_buffer
    }
}

/// A link layer frame type.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Type(pub u16);
impl Type {
    pub const RECEIVE_REQUEST: Self = Type(0x0148);
    pub const RECEIVE_RESPONSE: Self = Type(0x0149);
    pub const COMMAND_REQUEST: Self = Type(0x0B0F);
    pub const COMMAND_RESPONSE: Self = Type(0x0B10);
    pub const PING_REQUEST: Self = Type(0x0B00);
    pub const PING_RESPONSE: Self = Type(0x0B01);
    pub const ENUMERATION_START_REQUEST: Self = Type(0x0014);
    pub const ENUMERATION_START_RESPONSE: Self = Type(0x0015);
    pub const ENUMERATION_REQUEST: Self = Type(0x0038);
    pub const ENUMERATION_RESPONSE: Self = Type(0x0039);
    pub const ASSIGN_GATEWAY_ID_REQUEST: Self = Type(0x003C);
    pub const ASSIGN_GATEWAY_ID_RESPONSE: Self = Type(0x003D);
    pub const IDENTIFY_REQUEST: Self = Type(0x003A);
    pub const IDENTIFY_RESPONSE: Self = Type(0x003B);
    pub const VERSION_REQUEST: Self = Type(0x000A);
    pub const VERSION_RESPONSE: Self = Type(0x000B);
    pub const ENUMERATION_END_REQUEST: Self = Type(0x0E02);
    pub const ENUMERATION_END_RESPONSE: Self = Type(0x0006);
}

impl std::fmt::Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::RECEIVE_REQUEST => f.write_str("Type::RECEIVE_REQUEST"),
            Self::RECEIVE_RESPONSE => f.write_str("Type::RECEIVE_RESPONSE"),
            Self::COMMAND_REQUEST => f.write_str("Type::COMMAND_REQUEST"),
            Self::COMMAND_RESPONSE => f.write_str("Type::COMMAND_RESPONSE"),
            Self::PING_REQUEST => f.write_str("Type::PING_REQUEST"),
            Self::PING_RESPONSE => f.write_str("Type::PING_RESPONSE"),
            Self::ENUMERATION_START_REQUEST => f.write_str("Type::ENUMERATION_START_REQUEST"),
            Self::ENUMERATION_START_RESPONSE => f.write_str("Type::ENUMERATION_START_RESPONSE"),
            Self::ENUMERATION_REQUEST => f.write_str("Type::ENUMERATION_REQUEST"),
            Self::ENUMERATION_RESPONSE => f.write_str("Type::ENUMERATION_RESPONSE"),
            Self::ASSIGN_GATEWAY_ID_REQUEST => f.write_str("Type::ASSIGN_GATEWAY_ID_REQUEST"),
            Self::ASSIGN_GATEWAY_ID_RESPONSE => f.write_str("Type::ASSIGN_GATEWAY_ID_RESPONSE"),
            Self::IDENTIFY_REQUEST => f.write_str("Type::IDENTIFY_REQUEST"),
            Self::IDENTIFY_RESPONSE => f.write_str("Type::IDENTIFY_RESPONSE"),
            Self::VERSION_REQUEST => f.write_str("Type::VERSION_REQUEST"),
            Self::VERSION_RESPONSE => f.write_str("Type::VERSION_RESPONSE"),
            Self::ENUMERATION_END_REQUEST => f.write_str("Type::ENUMERATION_END_REQUEST"),
            Self::ENUMERATION_END_RESPONSE => f.write_str("Type::ENUMERATION_END_RESPONSE"),
            Self(value) => f
                .debug_tuple("Type")
                .field(&format_args!("{:#04x}", value))
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_encoding() {
        let encoded = Frame {
            address: Address::From(GatewayID::try_from(0x1201).unwrap()),
            frame_type: Type(0x0149),
            payload: b"\x00\xFF\x7C\xDB\xC2".as_slice().into(),
        }
        .encode();

        assert_eq!(
            encoded.as_slice(),
            [
                0xFF, 0x7E, 0x07, 0x92, 0x01, 0x01, 0x49, 0x00, 0xFF, 0x7C, 0xDB, 0xC2, 0x7E, 0x05,
                0x85, 0x7E, 0x08
            ]
            .as_slice()
        );

        assert!(encoded.capacity() <= encoded.len() + 6);
    }

    #[test]
    fn type_debug() {
        assert_eq!(
            format!("{:?}", &Type::RECEIVE_RESPONSE),
            "Type::RECEIVE_RESPONSE"
        );
        assert_eq!(format!("{:?}", &Type(0x1234)), "Type(0x1234)");
    }
}
