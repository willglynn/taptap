use crate::pv::application::NodeTableResponseEntry;
use crate::pv::network::NodeAddress;
use crate::pv::{LongAddress, NodeID};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct NodeTable(pub(crate) BTreeMap<NodeID, LongAddress>);

impl JsonSchema for NodeTable {
    fn schema_name() -> Cow<'static, str> {
        "NodeTable".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::NodeTable").into()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "array",
            "uniqueItems": true,
            "items": gen.subschema_for::<NodeTableEntry>(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, JsonSchema, Serialize, Deserialize)]
struct NodeTableEntry {
    pub node_id: NodeID,
    pub long_address: LongAddress,
}

impl From<(NodeID, LongAddress)> for NodeTableEntry {
    fn from((node_id, long_address): (NodeID, LongAddress)) -> Self {
        Self {
            node_id,
            long_address,
        }
    }
}
impl From<(&NodeID, &LongAddress)> for NodeTableEntry {
    fn from((&node_id, &long_address): (&NodeID, &LongAddress)) -> Self {
        Self {
            node_id,
            long_address,
        }
    }
}
impl From<NodeTableEntry> for (NodeID, LongAddress) {
    fn from(value: NodeTableEntry) -> Self {
        (value.node_id, value.long_address)
    }
}

// Serialize as Vec<Entry>
impl Serialize for NodeTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries: Vec<NodeTableEntry> = self.0.iter().map(NodeTableEntry::from).collect();
        entries.serialize(serializer)
    }
}

// Deserialize from Vec<Entry>
impl<'de> Deserialize<'de> for NodeTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = <Vec<NodeTableEntry>>::deserialize(deserializer)?;
        let len = entries.len();
        let output: Self = Self(
            entries
                .into_iter()
                .map(<(NodeID, LongAddress)>::from)
                .collect(),
        );

        if output.0.len() == len {
            Ok(output)
        } else {
            Err(D::Error::custom(
                "node_ids must be unique within a node table",
            ))
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct NodeTableBuilder {
    expected_next: Option<NodeID>,
    table: NodeTable,
}

impl NodeTableBuilder {
    pub fn push(
        &mut self,
        start_address: NodeAddress,
        entries: &[NodeTableResponseEntry],
    ) -> Option<NodeTable> {
        // Are we continuing an existing table?
        if NodeAddress::from(self.expected_next) != start_address {
            // Reset
            self.expected_next = Default::default();
            self.table = Default::default();
            if start_address == NodeAddress::ZERO {
                // We're mid-table
                // Ignore
                return None;
            }
        }

        // Insert all the records
        for entry in entries {
            let Ok(node_id) = entry.node_id.try_into() else {
                // Fail
                self.expected_next = None;
                return None;
            };

            // Insert the record
            self.table.0.insert(node_id, entry.long_address);
        }

        // This was the end of the table?
        if entries.is_empty() {
            // Take the table
            let mut table = Default::default();
            std::mem::swap(&mut self.table, &mut table);

            // Reset
            self.expected_next = None;

            // Return the table
            Some(table)
        } else {
            // There's more
            let last = self.table.0.last_entry().unwrap();
            self.expected_next = last.key().successor();

            // Did we wrap?
            if self.expected_next.is_none() {
                self.table = Default::default();
            }

            None
        }
    }
}
