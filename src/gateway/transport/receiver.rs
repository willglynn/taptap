use super::super::link::{self, Frame, GatewayID};
use super::*;
use crate::gateway::link::Address;
use crate::pv;
use crate::pv::link::SlotCounter;
use crate::pv::network::ReceivedPacketHeader;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::mem::size_of;

pub trait Sink {
    /// Enumeration started, using the indicated gateway ID.
    fn enumeration_started(&mut self, enumeration_gateway_id: GatewayID);

    /// A gateway's address was observed.
    ///
    /// If the network is enumerating, the gateway ID may be the `enumeration_gateway_id`, in which
    /// case this ID may not be unique.
    fn gateway_identity_observed(&mut self, gateway_id: GatewayID, address: pv::LongAddress);

    /// A gateway's version was observed.
    fn gateway_version_observed(&mut self, gateway_id: GatewayID, version: &str);

    /// Enumeration ended.
    fn enumeration_ended(&mut self, gateway_id: GatewayID);

    /// A gateway's slot counter was captured inside the gateway.
    ///
    /// The value of the slot counter at this moment may be described by a subsequent call to
    /// `gateway_slot_counter_observed()`.
    fn gateway_slot_counter_captured(&mut self, gateway_id: GatewayID);

    /// A gateway's slot counter was observed.
    ///
    /// The indicated slot counter value corresponds to the moment when the counter was most
    /// recently captured by the gateway, which occurred 4 to 50+ milliseconds ago.
    fn gateway_slot_counter_observed(&mut self, gateway_id: GatewayID, slot_counter: SlotCounter);

    /// A PV network packet was received from a gateway.
    fn packet_received(
        &mut self,
        gateway_id: GatewayID,
        header: &ReceivedPacketHeader,
        data: &[u8],
    );

    /// A command was executed by a gateway.
    fn command_executed(
        &mut self,
        gateway_id: GatewayID,
        request: (PacketType, &[u8]),
        response: (PacketType, &[u8]),
    );
}

#[derive(Debug, Clone)]
pub struct Receiver<S: Sink> {
    sink: S,
    rx_packet_numbers: BTreeMap<GatewayID, u16>,
    command_sequence_numbers: BTreeMap<GatewayID, CommandSequenceNumber>,
    commands_awaiting_response: BTreeMap<(GatewayID, CommandSequenceNumber), (PacketType, Vec<u8>)>,
    counters: Counters,
}

impl<S: Sink> link::Sink for Receiver<S> {
    fn frame(&mut self, frame: Frame) {
        match frame.frame_type {
            link::Type::RECEIVE_REQUEST => {
                self.receive_request(frame);
            }
            link::Type::RECEIVE_RESPONSE => {
                self.receive_response(frame);
            }
            link::Type::COMMAND_REQUEST => {
                self.command_request(frame);
            }
            link::Type::COMMAND_RESPONSE => {
                self.command_response(frame);
            }
            link::Type::PING_REQUEST => {
                self.counters.ping_requests += 1;
            }
            link::Type::PING_RESPONSE => {
                self.counters.ping_responses += 1;
            }
            link::Type::ENUMERATION_START_REQUEST => {
                self.enumeration_start_request(frame);
            }
            link::Type::ENUMERATION_START_RESPONSE => {
                self.counters.enumeration_start_responses += 1;
            }
            link::Type::ENUMERATION_REQUEST => {
                self.counters.enumeration_requests += 1;
            }
            link::Type::ENUMERATION_RESPONSE => {
                self.enumeration_response(frame);
            }
            link::Type::ASSIGN_GATEWAY_ID_REQUEST => {
                self.counters.assign_gateway_id_requests += 1;
            }
            link::Type::ASSIGN_GATEWAY_ID_RESPONSE => {
                self.counters.assign_gateway_id_responses += 1;
            }
            link::Type::IDENTIFY_REQUEST => {
                self.counters.identify_requests += 1;
            }
            link::Type::IDENTIFY_RESPONSE => {
                self.identify_response(frame);
            }
            link::Type::VERSION_REQUEST => {
                self.counters.version_requests += 1;
            }
            link::Type::VERSION_RESPONSE => {
                self.version_response(frame);
            }
            link::Type::ENUMERATION_END_REQUEST => {
                self.counters.enumeration_end_requests += 1;
            }
            link::Type::ENUMERATION_END_RESPONSE => match frame.address {
                Address::From(gateway) => {
                    self.counters.enumeration_end_responses += 1;
                    self.sink.enumeration_ended(gateway);
                }
                Address::To(_) => {
                    self.counters.invalid_enumeration_end_responses += 1;
                }
            },
            _ => {
                self.counters.unhandled_frame_type += 1;
            }
        }
    }
}

impl<S: Sink> Receiver<S> {
    /// Instantiate a new receiver with a given `Sink`.
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            rx_packet_numbers: Default::default(),
            command_sequence_numbers: Default::default(),
            commands_awaiting_response: Default::default(),
            counters: Default::default(),
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
        self.counters = Default::default();
    }

    fn receive_request(&mut self, frame: Frame) {
        let Address::To(gateway_id) = frame.address else {
            self.counters.invalid_receive_request += 1;
            return;
        };

        let Ok(payload) = ReceiveRequest::ref_from_bytes(frame.payload.as_ref()) else {
            self.counters.invalid_receive_request += 1;
            return;
        };

        // Indicate that the gateway captured its slot counter now, while processing the receive
        // request
        self.sink.gateway_slot_counter_captured(gateway_id);

        self.counters.receive_requests += 1;

        // Record the packet number for this gateway
        let n: u16 = payload.packet_number.into();
        *self.rx_packet_numbers.entry(gateway_id).or_insert(n) = n;
    }

    fn receive_response(&mut self, frame: Frame) {
        let Address::From(gateway_id) = frame.address else {
            self.counters.invalid_receive_responses += 1;
            return;
        };

        // Get the packet number for this gateway
        let Some(n) = self.rx_packet_numbers.get_mut(&gateway_id) else {
            self.counters.receive_response_from_unknown_gateway += 1;
            return;
        };

        // Interpret the response
        let Ok((status, packets)) = ReceiveResponse::read_from_bytes(frame.payload.as_ref(), *n)
        else {
            self.counters.invalid_receive_responses += 1;
            return;
        };

        self.counters.receive_responses += 1;

        // TODO: deduplicate gateway -> controller retransmissions

        // Update the packet number
        *n = status.packet_number;

        // Observe the slot counter
        self.sink
            .gateway_slot_counter_observed(gateway_id, status.slot_counter);

        for packet in packets {
            if let Ok((header, data)) = packet {
                self.counters.receive_packets += 1;

                // Observe the packet
                self.sink.packet_received(gateway_id, header, data);
            } else {
                self.counters.receive_packet_too_short += 1;
            }
        }
    }

    fn command_request(&mut self, frame: Frame) {
        let Address::To(gateway_id) = frame.address else {
            println!("bad tx request: {:?}", frame);
            self.counters.invalid_command_requests += 1;
            return;
        };

        if frame.payload.len() < size_of::<CommandRequest>() {
            println!("bad tx request: {:?}", frame);
            self.counters.invalid_command_requests += 1;
            return;
        }

        let (header, payload) = frame.payload.split_at(size_of::<CommandRequest>());
        let header = CommandRequest::ref_from_bytes(header).unwrap(); // infallible

        // The gateway may respond to this, so record it
        self.commands_awaiting_response.insert(
            (gateway_id, header.sequence_number),
            (header.packet_type, payload.to_vec()),
        );

        // Is this a retransmission from our vantage point?
        let retransmission = match self.command_sequence_numbers.entry(gateway_id) {
            Entry::Occupied(e) if *e.get() == header.sequence_number => true,
            Entry::Occupied(mut e) => {
                e.insert(header.sequence_number);
                false
            }
            Entry::Vacant(e) => {
                e.insert(header.sequence_number);
                false
            }
        };

        // Count it appropriately
        if retransmission {
            self.counters.retransmitted_command_requests += 1;
        } else {
            self.counters.command_requests += 1;
        }
    }

    fn command_response(&mut self, frame: Frame) {
        let Address::From(gateway_id) = frame.address else {
            println!("wrong addr: {:?}", frame);
            self.counters.invalid_command_responses += 1;
            return;
        };

        if frame.payload.len() < size_of::<CommandResponse>() {
            println!("bad tx response: {:?}", frame);
            self.counters.invalid_command_responses += 1;
            return;
        };

        let (header, payload) = frame.payload.split_at(size_of::<CommandResponse>());
        let header = CommandResponse::ref_from_bytes(header).unwrap(); // infallible

        // Deduplicate responses
        let Some((request_packet_type, request_payload)) = self
            .commands_awaiting_response
            .remove(&(gateway_id, header.command_sequence_number))
        else {
            self.counters.retransmitted_command_responses += 1;
            return;
        };

        self.counters.command_responses += 1;

        self.sink.command_executed(
            gateway_id,
            (request_packet_type, request_payload.as_slice()),
            (header.packet_type, payload),
        );
    }

    fn enumeration_start_request(&mut self, frame: Frame) {
        let Address::To(GatewayID::ZERO) = frame.address else {
            self.counters.invalid_enumeration_start_request += 1;
            return;
        };

        let Ok(request) = EnumerationStartRequest::ref_from_bytes(frame.payload.as_ref()) else {
            self.counters.invalid_enumeration_start_request += 1;
            return;
        };

        let Some(gateway_id) = request.enumeration_gateway_id() else {
            self.counters.invalid_enumeration_start_request += 1;
            return;
        };

        self.counters.enumeration_start_requests += 1;

        self.sink.enumeration_started(gateway_id);
    }

    fn identify_response(&mut self, frame: Frame) {
        let Address::From(gateway_id) = frame.address else {
            self.counters.invalid_identify_responses += 1;
            return;
        };

        let Ok(response) = IdentifyResponse::ref_from_bytes(frame.payload.as_ref()) else {
            self.counters.invalid_identify_responses += 1;
            return;
        };

        self.counters.identify_responses += 1;

        self.sink
            .gateway_identity_observed(gateway_id, response.pv_long_address);
    }

    fn enumeration_response(&mut self, frame: Frame) {
        let Address::From(gateway_id) = frame.address else {
            self.counters.invalid_enumeration_responses += 1;
            return;
        };

        let Ok(response) = IdentifyResponse::ref_from_bytes(frame.payload.as_ref()) else {
            self.counters.invalid_enumeration_responses += 1;
            return;
        };

        self.counters.enumeration_responses += 1;

        self.sink
            .gateway_identity_observed(gateway_id, response.pv_long_address);
    }

    pub fn version_response(&mut self, frame: Frame) {
        let Address::From(gateway_id) = frame.address else {
            self.counters.invalid_version_responses += 1;
            return;
        };

        let version = match std::str::from_utf8(frame.payload.as_ref()) {
            Ok(str) if !str.is_empty() => str,
            _ => {
                self.counters.invalid_version_responses += 1;
                return;
            }
        };

        self.counters.version_responses += 1;
        self.sink.gateway_version_observed(gateway_id, version);
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Counters {
    /// The number of received frames with an unknown frame type.
    pub unhandled_frame_type: u64,
    pub invalid_receive_request: u64,
    pub receive_requests: u64,
    pub invalid_receive_responses: u64,
    pub receive_response_from_unknown_gateway: u64,
    pub receive_responses: u64,
    pub receive_packets: u64,
    pub receive_packet_too_short: u64,
    pub invalid_command_requests: u64,
    pub retransmitted_command_requests: u64,
    pub command_requests: u64,
    pub invalid_command_responses: u64,
    pub retransmitted_command_responses: u64,
    pub command_responses: u64,
    pub ping_requests: u64,
    pub ping_responses: u64,
    pub enumeration_start_requests: u64,
    pub invalid_enumeration_start_request: u64,
    pub enumeration_start_responses: u64,
    pub enumeration_requests: u64,
    pub enumeration_responses: u64,
    pub invalid_enumeration_responses: u64,
    pub version_requests: u64,
    pub version_responses: u64,
    pub invalid_version_responses: u64,
    pub enumeration_end_requests: u64,
    pub enumeration_end_responses: u64,
    pub invalid_enumeration_end_responses: u64,
    pub assign_gateway_id_requests: u64,
    pub assign_gateway_id_responses: u64,
    pub identify_requests: u64,
    pub identify_responses: u64,
    pub invalid_identify_responses: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway;
    use crate::gateway::link::{Sink, Type};
    use crate::pv::LongAddress;

    #[derive(Debug, Clone, Eq, PartialEq)]
    enum Event {
        EnumerationStarted {
            enumeration_gateway_id: GatewayID,
        },
        GatewayIdentityObserved {
            gateway_id: GatewayID,
            address: LongAddress,
        },
        GatewayVersionObserved {
            gateway_id: GatewayID,
            version: String,
        },
        EnumerationEnded {
            gateway_id: GatewayID,
        },
        GatewaySlotCounterCaptured {
            gateway_id: GatewayID,
        },
        GatewaySlotCounterObserved {
            gateway_id: GatewayID,
            slot_counter: SlotCounter,
        },
        PacketReceived {
            gateway_id: GatewayID,
            header: ReceivedPacketHeader,
            data: Vec<u8>,
        },
        CommandExecuted {
            gateway_id: GatewayID,
            request: (PacketType, Vec<u8>),
            response: (PacketType, Vec<u8>),
        },
    }
    use Event::*;

    #[derive(Debug, Default)]
    struct TestSink(Vec<Event>);
    impl super::Sink for TestSink {
        fn enumeration_started(&mut self, enumeration_gateway_id: GatewayID) {
            self.0.push(EnumerationStarted {
                enumeration_gateway_id,
            });
        }

        fn gateway_identity_observed(&mut self, gateway_id: GatewayID, address: LongAddress) {
            self.0.push(GatewayIdentityObserved {
                gateway_id,
                address,
            })
        }

        fn gateway_version_observed(&mut self, gateway_id: GatewayID, version: &str) {
            self.0.push(GatewayVersionObserved {
                gateway_id,
                version: version.into(),
            })
        }

        fn enumeration_ended(&mut self, gateway_id: GatewayID) {
            self.0.push(EnumerationEnded { gateway_id });
        }

        fn gateway_slot_counter_captured(&mut self, gateway_id: GatewayID) {
            self.0.push(GatewaySlotCounterCaptured { gateway_id });
        }

        fn gateway_slot_counter_observed(
            &mut self,
            gateway_id: GatewayID,
            slot_counter: SlotCounter,
        ) {
            self.0.push(GatewaySlotCounterObserved {
                gateway_id,
                slot_counter,
            });
        }

        fn packet_received(
            &mut self,
            gateway_id: GatewayID,
            header: &ReceivedPacketHeader,
            data: &[u8],
        ) {
            self.0.push(PacketReceived {
                gateway_id,
                header: header.clone(),
                data: data.into(),
            })
        }

        fn command_executed(
            &mut self,
            gateway_id: GatewayID,
            request: (PacketType, &[u8]),
            response: (PacketType, &[u8]),
        ) {
            self.0.push(CommandExecuted {
                gateway_id,
                request: (request.0, request.1.into()),
                response: (response.0, response.1.into()),
            })
        }
    }

    #[test]
    fn unhandled_frame_type() {
        let mut rx = Receiver::new(TestSink::default());
        rx.frame(Frame {
            address: 0x1201.into(),
            frame_type: Type(0xffff),
            payload: vec![],
        });

        assert_eq!(&rx.sink().0, &[]);
        assert_eq!(
            rx.counters(),
            &Counters {
                unhandled_frame_type: 1,
                ..Default::default()
            }
        );
    }

    #[test]
    fn reset_counters() {
        let mut rx = Receiver::new(TestSink::default());

        assert_eq!(rx.counters(), &Counters::default());

        rx.frame(Frame {
            address: 0x1201.into(),
            frame_type: Type(0xffff),
            payload: vec![],
        });
        assert_ne!(rx.counters(), &Counters::default());

        rx.reset_counters();
        assert_eq!(rx.counters(), &Counters::default());
    }

    #[test]
    fn enumeration_sequence() {
        // Receive the exchange from the doc
        let mut rx = gateway::link::Receiver::new(Receiver::new(TestSink::default()));
        rx.extend_from_slice(crate::test_data::ENUMERATION_SEQUENCE);

        assert_eq!(
            &rx.sink().sink().0,
            &[
                EnumerationStarted {
                    enumeration_gateway_id: GatewayID::try_from(0x1235).unwrap()
                },
                EnumerationStarted {
                    enumeration_gateway_id: GatewayID::try_from(0x1235).unwrap()
                },
                EnumerationStarted {
                    enumeration_gateway_id: GatewayID::try_from(0x1235).unwrap()
                },
                EnumerationStarted {
                    enumeration_gateway_id: GatewayID::try_from(0x1235).unwrap()
                },
                EnumerationStarted {
                    enumeration_gateway_id: GatewayID::try_from(0x1235).unwrap()
                },
                GatewayIdentityObserved {
                    gateway_id: GatewayID::try_from(0x1235).unwrap(),
                    address: LongAddress([0x04, 0xC0, 0x5B, 0x30, 0x00, 0x02, 0xBE, 0x16])
                },
                GatewayIdentityObserved {
                    gateway_id: GatewayID::try_from(0x1201).unwrap(),
                    address: LongAddress([0x04, 0xC0, 0x5B, 0x30, 0x00, 0x02, 0xBE, 0x16])
                },
                GatewayIdentityObserved {
                    gateway_id: GatewayID::try_from(0x1202).unwrap(),
                    address: LongAddress([0x04, 0xC0, 0x5B, 0x30, 0x00, 0x02, 0xBE, 0x16])
                },
                GatewayIdentityObserved {
                    gateway_id: GatewayID::try_from(0x1201).unwrap(),
                    address: LongAddress([0x04, 0xC0, 0x5B, 0x30, 0x00, 0x02, 0xBE, 0x16])
                },
                GatewayVersionObserved {
                    gateway_id: GatewayID::try_from(0x1201).unwrap(),
                    version: "Mgate Version G8.59\rJul  6 2020\r16:51:51\rGW-H158.4.3S0.12\r"
                        .into()
                },
                EnumerationEnded {
                    gateway_id: GatewayID::try_from(0x1201).unwrap()
                },
            ]
        );
        assert_eq!(
            rx.sink().counters(),
            &Counters {
                unhandled_frame_type: 2,
                ping_requests: 2,
                ping_responses: 2,
                enumeration_start_requests: 5,
                enumeration_start_responses: 5,
                enumeration_requests: 6,
                enumeration_responses: 1,
                version_requests: 1,
                version_responses: 1,
                enumeration_end_requests: 1,
                enumeration_end_responses: 1,
                assign_gateway_id_requests: 2,
                assign_gateway_id_responses: 2,
                identify_requests: 3,
                identify_responses: 3,
                ..Default::default()
            }
        );
    }
}
