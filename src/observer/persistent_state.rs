use crate::barcode::Barcode;
use crate::gateway::link::GatewayID;
use crate::observer::node_table::NodeTable;
use crate::pv::{LongAddress, NodeID};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Persistent state of an observed network.
///
/// Information like hardware addresses and version numbers are exchanged infrequently. This data
/// is captured and stored in `PersistentState`.
#[derive(Debug, Clone, Eq, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct PersistentState {
    pub gateway_node_tables: BTreeMap<GatewayID, NodeTable>,
    pub gateway_identities: BTreeMap<GatewayID, LongAddress>,
    pub gateway_versions: BTreeMap<GatewayID, String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistentStateEventGateway {
    pub address: String,
    pub version: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistentStateEventNode {
    pub address: String,
    pub barcode: Barcode,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistentStateEvent {
    pub event_type: String,
    pub gateways: BTreeMap<GatewayID, PersistentStateEventGateway>,
    pub nodes: BTreeMap<GatewayID, BTreeMap<NodeID, PersistentStateEventNode>>,
}

impl From<&PersistentState> for PersistentStateEvent {
    fn from(item: &PersistentState) -> Self {
        let event_type = "infrastructure_report".to_string();
        let gateways = item
            .gateway_identities
            .iter()
            .map(|(gateway_id, long_address)| {
                let version = item
                    .gateway_versions
                    .get(gateway_id)
                    .cloned()
                    .unwrap_or_default();
                let address = format!(
                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    long_address.0[0],
                    long_address.0[1],
                    long_address.0[2],
                    long_address.0[3],
                    long_address.0[4],
                    long_address.0[5],
                    long_address.0[6],
                    long_address.0[7]
                );
                (
                    *gateway_id,
                    PersistentStateEventGateway {
                        address,
                        version: version.clone(),
                    },
                )
            })
            .collect();

        let nodes = item
            .gateway_node_tables
            .iter()
            .map(|(gateway_id, table)| {
                let nodes = table
                    .0
                    .iter()
                    .map(|(node_id, long_address)| {
                        let address = format!(
                            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                            long_address.0[0],
                            long_address.0[1],
                            long_address.0[2],
                            long_address.0[3],
                            long_address.0[4],
                            long_address.0[5],
                            long_address.0[6],
                            long_address.0[7]
                        );
                        let barcode = long_address.into();
                        (*node_id, PersistentStateEventNode { address, barcode })
                    })
                    .collect();
                (*gateway_id, nodes)
            })
            .collect();

        PersistentStateEvent {
            event_type,
            gateways,
            nodes,
        }
    }
}

impl JsonSchema for PersistentStateEvent {
    fn schema_name() -> Cow<'static, str> {
        "PersistentState".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::PersistentState").into()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "required": ["event_type", "gateways", "nodes"],
            "properties": {
                "event_type": {
                    "type": "string"
                },
                "gateways": {
                    "type": "object",
                    "properties": {
                        "propertyNames": gen.subschema_for::<GatewayID>(),
                        "additionalProperties": {
                            "type": "object",
                            "required": ["address", "version"],
                            "properties": {
                                "address": {
                                    "type": "string",
                                    "pattern": "^[0-9A-F]{2}(:[0-9A-F]{2}){7}$"
                                },
                                "version": {
                                    "type": "string"
                                }
                            }
                        }
                    }
                },
                "nodes": {
                    "type": "object",
                    "properties": {
                        "propertyNames": gen.subschema_for::<NodeID>(),
                        "additionalProperties": {
                            "type": "object",
                            "required": ["address", "version"],
                            "properties": {
                                "address": {
                                    "type": "string",
                                    "pattern": "^[0-9A-F]{2}(:[0-9A-F]{2}){7}$"
                                },
                                "barcode": {
                                    "type": "string"
                                }
                            }
                        }
                    }
                },
            }
        })
    }
}
