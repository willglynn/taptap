use super::*;
use crate::gateway::GatewayID;
use crate::pv::network::{NodeAddress, ReceivedPacketHeader};
use crate::pv::{LongAddress, NodeID, PacketType, SlotCounter};
use crate::{gateway, pv};

pub trait Sink {
    fn string_request(&mut self, gateway_id: GatewayID, pv_node_id: pv::NodeID, request: &str);
    fn string_response(&mut self, gateway_id: GatewayID, pv_node_id: pv::NodeID, response: &str);
    fn node_table_page(
        &mut self,
        gateway_id: GatewayID,
        start_address: NodeAddress,
        nodes: &[NodeTableResponseEntry],
    );

    fn topology_report(
        &mut self,
        gateway_id: GatewayID,
        pv_node_id: pv::NodeID,
        topology_report: &TopologyReport,
    );
    fn power_report(
        &mut self,
        gateway_id: GatewayID,
        pv_node_id: pv::NodeID,
        power_report: &PowerReport,
    );
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Counters {
    invalid_received_packet_node_ids: u64,
    invalid_power_reports: u64,
    power_reports: u64,
    invalid_topology_reports: u64,
    topology_reports: u64,
    invalid_node_table_requests: u64,
    invalid_node_table_responses: u64,
    invalid_string_commands: u64,
    string_commands: u64,
    invalid_string_responses: u64,
    string_responses: u64,
}

#[derive(Debug)]
pub struct Receiver<S: gateway::transport::Sink + Sink> {
    sink: S,
    counters: Counters,
}

impl<S: gateway::transport::Sink + Sink> Receiver<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            counters: Default::default(),
        }
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    pub fn into_inner(self) -> S {
        self.sink
    }

    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    fn node_table_command(&mut self, gateway_id: GatewayID, request: &[u8], response: &[u8]) {
        let Ok(request) = NodeTableRequest::ref_from_bytes(request) else {
            self.counters.invalid_node_table_requests += 1;
            return;
        };

        let Ok(response) = NodeTableResponse::ref_from_bytes(response) else {
            self.counters.invalid_node_table_responses += 1;
            return;
        };

        if response.entries.len() != response.entries_count.get() as usize {
            self.counters.invalid_node_table_responses += 1;
            return;
        };

        self.sink
            .node_table_page(gateway_id, request.start_at, &response.entries);
    }

    fn string_command(&mut self, gateway_id: GatewayID, request: &[u8], response: &[u8]) {
        let Ok((node, request)) = NodeAddress::ref_from_prefix(request) else {
            self.counters.invalid_string_commands += 1;
            return;
        };
        let Ok(node) = NodeID::try_from(*node) else {
            self.counters.invalid_string_commands += 1;
            return;
        };

        let Ok(request) = std::str::from_utf8(request) else {
            self.counters.invalid_string_commands += 1;
            return;
        };

        if !response.is_empty() {
            self.counters.invalid_string_commands += 1;
            return;
        }

        self.counters.string_commands += 1;

        self.sink.string_request(gateway_id, node, request);
    }
}

impl<S: gateway::transport::Sink + Sink> gateway::transport::Sink for Receiver<S> {
    fn enumeration_started(&mut self, enumeration_gateway_id: GatewayID) {
        self.sink.enumeration_started(enumeration_gateway_id)
    }

    fn gateway_identity_observed(&mut self, gateway_id: GatewayID, address: LongAddress) {
        self.sink.gateway_identity_observed(gateway_id, address)
    }

    fn gateway_version_observed(&mut self, gateway_id: GatewayID, version: &str) {
        self.sink.gateway_version_observed(gateway_id, version)
    }

    fn enumeration_ended(&mut self, gateway_id: GatewayID) {
        self.sink.enumeration_ended(gateway_id)
    }

    fn gateway_slot_counter_captured(&mut self, gateway_id: GatewayID) {
        self.sink.gateway_slot_counter_captured(gateway_id)
    }

    fn gateway_slot_counter_observed(&mut self, gateway_id: GatewayID, slot_counter: SlotCounter) {
        self.sink
            .gateway_slot_counter_observed(gateway_id, slot_counter)
    }

    fn packet_received(
        &mut self,
        gateway_id: GatewayID,
        header: &ReceivedPacketHeader,
        data: &[u8],
    ) {
        self.sink.packet_received(gateway_id, header, data);

        let Ok(node_id) = pv::NodeID::try_from(header.node_address) else {
            self.counters.invalid_received_packet_node_ids += 1;
            return;
        };

        match header.packet_type {
            PacketType::STRING_RESPONSE => {
                if let Ok(response) = std::str::from_utf8(data) {
                    self.counters.string_responses += 1;
                    self.sink.string_response(gateway_id, node_id, response);
                } else {
                    self.counters.invalid_string_responses += 1;
                }
            }
            PacketType::TOPOLOGY_REPORT => {
                if let Ok(topology_report) = TopologyReport::ref_from_bytes(data) {
                    self.counters.topology_reports += 1;
                    self.sink
                        .topology_report(gateway_id, node_id, topology_report);
                } else {
                    self.counters.invalid_topology_reports += 1;
                }
            }
            PacketType::POWER_REPORT => {
                if let Ok(power_report) = PowerReport::ref_from_bytes(data) {
                    self.counters.power_reports += 1;
                    self.sink.power_report(gateway_id, node_id, power_report);
                } else {
                    self.counters.invalid_power_reports += 1;
                }
            }
            _ => {}
        }
    }

    fn command_executed(
        &mut self,
        gateway_id: GatewayID,
        request: (PacketType, &[u8]),
        response: (PacketType, &[u8]),
    ) {
        self.sink.command_executed(gateway_id, request, response);

        match (request.0, response.0) {
            (PacketType::NODE_TABLE_REQUEST, PacketType::NODE_TABLE_RESPONSE) => {
                self.node_table_command(gateway_id, request.1, response.1);
            }

            (PacketType::STRING_REQUEST, PacketType::STRING_RESPONSE) => {
                self.string_command(gateway_id, request.1, response.1);
            }
            //(PacketType::BROADCAST, PacketType::BROADCAST_ACK) => {}
            (
                PacketType::NETWORK_STATUS_REQUEST | PacketType::LONG_NETWORK_STATUS_REQUEST,
                PacketType::NETWORK_STATUS_RESPONSE,
            ) => {
                // TODO
            }
            _ => {
                /*
                eprintln!(
                    "unhandled command: {} ({} bytes) => {} ({} bytes)",
                    request.0,
                    request.1.len(),
                    response.0,
                    response.1.len()
                );
                 */
            }
        }
    }
}
