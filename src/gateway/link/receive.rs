use super::*;

/// An object which handles reception callbacks.
pub trait Sink {
    fn frame(&mut self, frame: Frame);
}

impl Sink for Vec<Frame> {
    fn frame(&mut self, frame: Frame) {
        self.push(frame.clone());
    }
}

/// A receiver which converts a series of bytes into a series of `Frame`s.
///
/// The receiver tolerates line errors and attempts to re-synchronize whenever possible. Errors are
/// reported by incrementing counters.
#[derive(Debug)]
pub struct Receiver<S: Sink> {
    sink: S,
    state: State,
    counters: Counters,
    buffer: Vec<u8>,
}

impl<S: Sink> Receiver<S> {
    const MAX_FRAME_SIZE: usize = 256;

    /// Instantiate a new receiver with a given `Sink`.
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            state: Default::default(),
            counters: Default::default(),
            buffer: Default::default(),
        }
    }

    /// Access the `Sink`.
    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// Mutably access the `Sink`.
    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    /// Destroy the `Receiver` to obtain the `Sink`.
    pub fn into_inner(self) -> S {
        self.sink
    }

    /// Retrieve the current counters describing the receiver's activity.
    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    /// Reset the counters.
    pub fn reset_counters(&mut self) {
        self.counters = Counters::default();
    }

    /// Add a slice of bytes to the receiver.
    ///
    /// The receiver processes these bytes and calls functions on `Sink`.
    pub fn extend_from_slice(&mut self, buffer: &[u8]) {
        for byte in buffer {
            self.push_u8(*byte);
        }
    }

    /// Add a single byte to the receiver.
    fn push_u8(&mut self, byte: u8) {
        let next_state = match self.state {
            State::Idle => {
                match byte {
                    // Preamble, expected
                    0x00 | 0xff => State::Idle,
                    0x7e => State::StartOfFrame,
                    _ => State::Noise,
                }
            }
            State::Noise => {
                match byte {
                    // Preamble, expected
                    0x00 | 0xff => State::Idle,
                    // Possible start of frame
                    0x7e => State::StartOfFrame,
                    // Discard
                    _ => State::Noise,
                }
            }
            State::StartOfFrame => {
                match byte {
                    // Proper start of frame
                    0x07 => State::Frame,
                    // Improper
                    _ => State::Noise,
                }
            }
            State::Frame => {
                match byte {
                    // Escape sequence
                    0x7e => State::FrameEscape,
                    // Normal data byte
                    _ if self.buffer.len() < Self::MAX_FRAME_SIZE => {
                        self.buffer.push(byte);
                        State::Frame
                    }
                    // Overlong frame
                    _ => State::Giant,
                }
            }
            State::FrameEscape => {
                if byte == 0x08 {
                    // End of frame
                    self.parse_frame_from_buffer();
                    self.buffer.truncate(0);
                    State::Idle
                } else if let Ok(byte) = escaping::unescaped_byte(byte) {
                    if self.buffer.len() < Self::MAX_FRAME_SIZE {
                        self.buffer.push(byte);
                        State::Frame
                    } else {
                        self.buffer.truncate(0);
                        State::GiantEscape
                    }
                } else {
                    self.buffer.truncate(0);
                    State::Noise
                }
            }
            State::Giant => match byte {
                0x7e => State::GiantEscape,
                _ => State::Giant,
            },
            State::GiantEscape => {
                match byte {
                    // Start of frame
                    0x07 => State::Frame,
                    // End of frame
                    0x08 => State::Idle,
                    // Continue discarding
                    _ => State::Giant,
                }
            }
        };

        match next_state {
            State::Noise if self.state != State::Noise => {
                self.counters.noise += 1;
            }
            State::Giant if self.state != State::Giant && self.state != State::GiantEscape => {
                self.buffer.truncate(0);
                self.counters.giants += 1;
            }
            _ => {}
        }

        self.state = next_state;
    }

    fn parse_frame_from_buffer(&mut self) {
        // Ensure we're a valid length
        if self.buffer.len() < 6 {
            self.counters.runts += 1;
            return;
        }

        // Verify the CRC
        let (body, expected_crc) = self.buffer.split_at(self.buffer.len() - 2);
        let crc = crc::crc(body);
        let expected_crc = u16::from_le_bytes([expected_crc[0], expected_crc[1]]);
        if expected_crc != crc {
            self.counters.checksums += 1;
            return;
        }

        let address = Address::from([body[0], body[1]]);
        let frame_type = Type(u16::from_be_bytes([body[2], body[3]]));

        self.counters.frames += 1;
        self.sink.frame(Frame {
            address,
            frame_type,
            payload: Vec::from(body.split_at(4).1),
        });
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
enum State {
    #[default]
    Idle,
    Noise,
    StartOfFrame,
    Frame,
    FrameEscape,
    Giant,
    GiantEscape,
}

/// Counters describing the internal state transitions of a `Receiver`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Counters {
    /// The number of valid frames successfully received.
    pub frames: u64,
    /// The number of frames discarded for being too short.
    pub runts: u64,
    /// The number of frames discarded for being too long.
    pub giants: u64,
    /// The number of frames discarded for having an incorrect checksum.
    pub checksums: u64,
    /// The number of inter-frame periods where line noise was detected.
    pub noise: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path() {
        let mut rx = Receiver::new(Vec::new());
        rx.extend_from_slice(&[
            0x00, 0xFF, 0xFF, 0x7E, 0x07, 0x12, 0x01, 0x01, 0x48, 0x00, 0x01, 0x18, 0x83, 0x04,
            0x17, 0x44, 0x7E, 0x08, 0xFF, 0x7E, 0x07, 0x92, 0x01, 0x01, 0x49, 0x00, 0xFE, 0x01,
            0x83, 0x5A, 0xDE, 0x07, 0x00, 0x0A, 0x01, 0x14, 0x63, 0x3A, /*…*/ 0x79, 0x26,
            0x7E, 0x08, 0x00, 0xFF, 0xFF, 0x7E, 0x07, 0x12, 0x01, 0x01, 0x48, 0x00, 0x01, 0x18,
            0x84, 0x04, 0x1F, 0x09, 0x7E, 0x08, 0xFF, 0x7E, 0x07, 0x92, 0x01, 0x01, 0x49, 0x00,
            0xFF, 0x7C, 0xDB, 0xC2, 0x7E, 0x05, 0x85, 0x7E, 0x08,
        ]);
        assert_eq!(rx.state, State::Idle);
        assert_eq!(
            rx.counters,
            Counters {
                frames: 4,
                runts: 0,
                giants: 0,
                checksums: 0,
                noise: 0,
            }
        );
        assert_eq!(rx.buffer.len(), 0);

        assert_eq!(
            rx.sink,
            vec![
                Frame {
                    address: Address::To(0x1201.try_into().unwrap()),
                    frame_type: Type::RECEIVE_REQUEST,
                    payload: b"\x00\x01\x18\x83\x04".as_slice().into(),
                },
                Frame {
                    address: Address::From(0x1201.try_into().unwrap()),
                    frame_type: Type::RECEIVE_RESPONSE,
                    payload: b"\x00\xFE\x01\x83\x5A\xDE\x07\x00\x0A\x01\x14\x63\x3A".as_slice().into(),
                },
                Frame {
                    address: Address::To(0x1201.try_into().unwrap()),
                    frame_type: Type::RECEIVE_REQUEST,
                    payload: b"\x00\x01\x18\x84\x04".as_slice().into(),
                },
                Frame {
                    address: Address::From(0x1201.try_into().unwrap()),
                    frame_type: Type::RECEIVE_RESPONSE,
                    payload: b"\x00\xFF\x7C\xDB\xC2".as_slice().into(),
                },
            ]
        );
    }

    #[test]
    fn interframe_noise() {
        let mut rx = Receiver::new(Vec::new());
        rx.extend_from_slice(&[
            0xee, 0xee, 0xee, 0x00, 0xFF, 0xFF, 0x7E, 0x07, 0x12, 0x01, 0x01, 0x48, 0x00, 0x01,
            0x18, 0x83, 0x04, 0x17, 0x44, 0x7E, 0x08, 0x01, 0xFF, 0x7E, 0x07, 0x92, 0x01, 0x01,
            0x49, 0x00, 0xFE, 0x01, 0x83, 0x5A, 0xDE, 0x07, 0x00, 0x0A, 0x01, 0x14, 0x63, 0x3A,
            /*…*/ 0x79, 0x26, 0x7E, 0x08, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x00,
            0xFF, 0xFF, 0x7E, 0x07, 0x12, 0x01, 0x01, 0x48, 0x00, 0x01, 0x18, 0x84, 0x04, 0x1F,
            0x09, 0x7E, 0x08, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xFF, 0x7E,
            0x07, 0x92, 0x01, 0x01, 0x49, 0x00, 0xFF, 0x7C, 0xDB, 0xC2, 0x7E, 0x05, 0x85, 0x7E,
            0x08,
        ]);
        assert_eq!(rx.state, State::Idle);
        assert_eq!(
            rx.counters,
            Counters {
                frames: 4,
                runts: 0,
                giants: 0,
                checksums: 0,
                noise: 3,
            }
        );
        assert_eq!(rx.buffer.len(), 0);
    }

    #[test]
    fn checksum() {
        let mut rx = Receiver::new(Vec::new());
        rx.extend_from_slice(&[
            0x00, 0xFF, 0xFF, 0x7E, 0x07, 0x12, 0x01, 0x01, 0x48, 0x00, 0x01, 0x18, 0x83, 0x04,
            0x17, 0x44, 0x7E, 0x08, 0xFF, 0x7E, 0x07, 0x92, 0x01, 0x01, 0x49, 0x00, 0xFE, 0x01,
            0x83, 0x5A, 0xDE, 0x07, 0x00, 0x0A, 0x01, 0x14, 0x63, 0x3A, /*…*/ 0x79, 0x25,
            0x7E, 0x08, 0x00, 0xFF, 0xFF, 0x7E, 0x07, 0x12, 0x01, 0x01, 0x48, 0x00, 0x01, 0x18,
            0x84, 0x04, 0x1e, 0x09, 0x7E, 0x08, 0xFF, 0x7E, 0x07, 0x92, 0x01, 0x01, 0x49, 0x00,
            0xFF, 0x7C, 0xDB, 0xC2, 0x7E, 0x05, 0x85, 0x7E, 0x08,
        ]);
        assert_eq!(rx.state, State::Idle);
        assert_eq!(
            rx.counters,
            Counters {
                frames: 2,
                runts: 0,
                giants: 0,
                checksums: 2,
                noise: 0,
            }
        );
        assert_eq!(rx.buffer.len(), 0);
    }

    #[test]
    fn intraframe_noise() {
        let mut rx = Receiver::new(Vec::new());
        rx.extend_from_slice(&[
            0x00, 0xFF, 0xFF, 0x7E, 0x07, 0x12, 0x01, 0x01, 0x48, 0x00, 0x01, 0x7e, 0x83, 0x04,
            0x17, 0x44, 0x7E, 0x08, 0xFF, 0x7E, 0x07, 0x92, 0x01, 0x01, 0x49, 0x00, 0xFE, 0x01,
            0x83, 0x5A, 0xDE, 0x07, 0x00, 0x0A, 0x01, 0x14, 0x63, 0x3A, /*…*/ 0x79, 0x7e,
            0x7E, 0x08, 0x00, 0xFF, 0xFF, 0x7E, 0x7e, 0x12, 0x01, 0x01, 0x48, 0x00, 0x01, 0x18,
            0x84, 0x04, 0x1F, 0x09, 0x7E, 0x08, 0xFF, 0x7E, 0x07, 0x92, 0x01, 0x01, 0x49, 0x00,
            0xFF, 0x7C, 0xDB, 0xC2, 0x7E, 0x05, 0x85, 0x7E, 0x08,
        ]);
        assert_eq!(rx.state, State::Idle);
        assert_eq!(
            rx.counters,
            Counters {
                frames: 1,
                runts: 0,
                giants: 0,
                checksums: 0,
                noise: 6,
            }
        );
        assert_eq!(rx.buffer.len(), 0);
    }

    #[test]
    fn runt() {
        let mut rx = Receiver::new(Vec::new());

        let mut buf = Vec::new();
        buf.extend_from_slice(&[0xff, 0x7e, 0x07, 0x7e, 0x08]);
        buf.extend_from_slice(&[0x7e, 0x08]);

        rx.extend_from_slice(&[
            // underlength frames
            0xFF, 0x7E, 0x07, 0x7E, 0x08, 0xFF, 0x7E, 0x07, 0x00, 0x7E, 0x08, 0xFF, 0x7E, 0x07,
            0x00, 0x00, 0x7E, 0x08, 0xFF, 0x7E, 0x07, 0x00, 0x00, 0x00, 0x7E, 0x08, 0xFF, 0x7E,
            0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x08, // minimum length frame
            0xFF, 0x7E, 0x07, 0x00, 0x01, 0x00, 0x00, 0x89, 0xD0, 0x7E, 0x08,
        ]);
        assert_eq!(rx.state, State::Idle);
        assert_eq!(
            rx.counters,
            Counters {
                frames: 1,
                runts: 5,
                giants: 0,
                checksums: 0,
                noise: 0,
            }
        );
        assert_eq!(rx.buffer.len(), 0);
    }

    #[test]
    fn giant() {
        let mut rx = Receiver::new(Vec::new());
        rx.extend_from_slice(&[
            0x00, 0xFF, 0xFF, 0x7E, 0x07, 0x12, 0x01,
        ]);
        rx.extend_from_slice(&vec![0u8; 1000]);
        assert_eq!(rx.state, State::Giant);
        rx.extend_from_slice(&[0x7E]);
        assert_eq!(rx.state, State::GiantEscape);
        rx.extend_from_slice(&[0x08]);
        assert_eq!(rx.state, State::Idle);
        assert_eq!(
            rx.counters,
            Counters {
                frames: 0,
                runts: 0,
                giants: 1,
                checksums: 0,
                noise: 0,
            }
        );
        assert_eq!(rx.buffer.len(), 0);
    }
}
