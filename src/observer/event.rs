use super::*;
use crate::barcode::Barcode;
use crate::pv;
use crate::pv::link::InvalidSlotNumber;
use crate::pv::physical::RSSI;
use chrono::{DateTime, Local};

/// An event produced by an observer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename = "snake_case")]
pub enum Event {
    PowerReport(PowerReportEvent),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Gateway {
    /// The gateway's link layer ID.
    ///
    /// This value can change over time and is duplicated between different systems, but it is
    /// always present.
    pub id: gateway::link::GatewayID,

    /// The gateway's hardware address.
    ///
    /// This value is permanent and globally unique, but it is not always known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<pv::LongAddress>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Node {
    /// The node's ID.
    ///
    /// This value can change over time and is duplicated between different gateways, but it is
    /// always present.
    pub id: pv::NodeID,

    /// The node's hardware address.
    ///
    /// This value is permanent and globally unique, but it is not always known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<pv::LongAddress>,

    /// The node's barcode.
    ///
    /// This value is permanent and globally unique, but it is not always known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub barcode: Option<Barcode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PowerReportEvent {
    pub event_type: String,
    /// The gateway through which the power report was received.
    pub gateway: GatewayID,
    /// The node sending the power report.
    pub node: NodeID,
    /// The time at which this measurement was taken.
    pub timestamp: DateTime<Local>,
    pub voltage_in: f64,
    pub voltage_out: f64,
    pub current: f64,
    pub dc_dc_duty_cycle: f64,
    pub temperature: f64,
    pub rssi: RSSI,
}

impl PowerReportEvent {
    pub fn new(
        gateway: GatewayID,
        node: NodeID,
        slot_clock: &SlotClock,
        report: &pv::application::PowerReport,
    ) -> Result<Self, InvalidSlotNumber> {
        let timestamp = slot_clock.get(report.slot_counter)?;

        let (voltage_in, voltage_out) = report.voltage_in_and_voltage_out.into();
        let (current, temperature) = report.current_and_temperature.into();

        // XXX: is it correct to sign-extend temperature?
        // How are below-freezing temperatures reported? (This assumes two's complement.)
        let temperature = if temperature & 0x800 == 0 {
            temperature
        } else {
            temperature | 0xF000
        } as i16;

        Ok(Self {
            event_type: "power_report".to_string(),
            gateway,
            node,
            timestamp: timestamp.into(),
            voltage_in: voltage_in as f64 / 20.0,   //* 0.05,
            voltage_out: voltage_out as f64 / 10.0, // * 0.10,
            dc_dc_duty_cycle: report.dc_dc_duty_cycle as f64 / 255.0,
            current: current as f64 / 200.0,        // * 0.005,
            temperature: temperature as f64 / 10.0, // * 0.01,
            rssi: report.rssi,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pv::application::{PowerReport, U12Pair};

    #[test]
    fn negative_temperature() {
        let event_type = "power_report".to_string();
        let gateway = GatewayID::try_from(1).unwrap();
        let node = NodeID::try_from(1).unwrap();
        let rssi = RSSI(100);
        let timestamp = SystemTime::now();
        let slot_counter = SlotCounter::from(0);
        let slot_clock = SlotClock::new(slot_counter, timestamp).unwrap();

        let power_report = PowerReport {
            voltage_in_and_voltage_out: U12Pair::try_from((500, 250)).unwrap(),
            dc_dc_duty_cycle: 255,
            current_and_temperature: U12Pair::try_from((200, 0xfff)).unwrap(),
            unknown: [0, 0, 0],
            slot_counter,
            rssi,
        };

        let power_report_event =
            PowerReportEvent::new(gateway, node, &slot_clock, &power_report).unwrap();

        let actual = serde_json::to_string(&power_report_event).unwrap();
        let expected = serde_json::to_string(&PowerReportEvent {
            event_type,
            gateway,
            node,
            timestamp: timestamp.into(),
            voltage_in: 25.0,
            voltage_out: 25.0,
            current: 1.00,
            dc_dc_duty_cycle: 1.0,
            temperature: -0.1,
            rssi,
        })
        .unwrap();
        assert_eq!(actual, expected); // floats :|
    }
}
