use clap::{Args, Parser, Subcommand};
use log::LevelFilter;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::exit;
use taptap::gateway::physical::Connection;
use taptap::gateway::{physical, Frame, GatewayID};
use taptap::pv::application::{NodeTableResponseEntry, PowerReport, TopologyReport};
use taptap::pv::network::{NodeAddress, ReceivedPacketHeader};
use taptap::pv::{LongAddress, NodeID, PacketType, SlotCounter};
use taptap::{config, gateway, pv};

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    #[cfg(feature = "serialport")]
    ListSerialPorts,

    /// Observe the system, extracting data as it runs
    Observe {
        #[command(flatten)]
        source: Source,
    },

    /// Peek at the raw data flowing at the gateway physical layer
    PeekBytes {
        #[command(flatten)]
        source: Source,
        /// Print raw binary bytes without escaping
        #[arg(long)]
        raw: bool,
    },

    /// Peek at the assembled frames at the gateway link layer
    PeekFrames {
        #[command(flatten)]
        source: Source,
    },

    /// Peek at the gateway transport and PV application layer activity
    PeekActivity {
        #[command(flatten)]
        source: Source,
    },
}

#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = true)]
struct Source {
    /// The name of the serial port (try `taptap list-serial-ports`)
    #[arg(long, group = "mode", value_name = "SERIAL-PORT")]
    #[cfg(feature = "serialport")]
    serial: Option<String>,

    /// The IP or hostname which is providing serial-over-TCP service
    #[arg(long, group = "mode", value_name = "DESTINATION")]
    tcp: Option<String>,

    // If --tcp is specified, the port to which to connect
    #[arg(long, requires = "tcp", default_value_t = 7160)]
    port: u16,
}

impl Source {
    fn open(&self) -> Box<dyn physical::Connection> {
        let src = config::SourceConfig::from(self.clone());
        match src.open() {
            Ok(s) => s,
            Err(e) => {
                log::error!("error opening source: {}", e);
                exit(2);
            }
        }
    }
}

impl From<Source> for config::SourceConfig {
    fn from(value: Source) -> Self {
        #[cfg(feature = "serialport")]
        if let Some(name) = value.serial {
            return config::SerialSourceConfig { name }.into();
        }

        match (value.tcp,) {
            (Some(name),) => config::TcpConnectionConfig {
                hostname: name,
                port: value.port,
                mode: config::ConnectionMode::ReadOnly,
            }
            .into(),
            _ => {
                // clap assertions should prevent this
                panic!("a source must be specified");
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    match cli.command {
        Commands::PeekBytes { source, raw } => {
            let source = source.open();
            peek_bytes(source, raw);
        }

        Commands::PeekFrames { source } => {
            let source = source.open();
            peek_frames(source);
        }

        Commands::PeekActivity { source } => {
            let source = source.open();
            peek_activity(source);
        }

        #[cfg(feature = "serialport")]
        Commands::ListSerialPorts => {
            list_serial_ports();
        }

        Commands::Observe { source } => {
            let source = source.open();
            observe(source)
        }
    }
}

fn peek_bytes(mut conn: Box<dyn physical::Connection>, raw: bool) {
    let mut buffer = [0u8; 1024];
    let mut last_was_7e = false;

    loop {
        let slice = match conn.read(&mut buffer) {
            Ok(n) => &buffer[0..n],
            Err(e) => {
                log::error!("error reading: {}", e);
                exit(1);
            }
        };

        if slice.is_empty() {
            return;
        }

        let mut out = std::io::stdout().lock();
        if raw {
            out.write_all(slice).unwrap();
        } else {
            let mut formatted = Vec::with_capacity(4 * slice.len());
            for byte in slice {
                let sep = if last_was_7e && *byte == 0x08 {
                    '\n'
                } else {
                    ' '
                };
                write!(&mut formatted, "{:02X}{}", byte, sep).unwrap();
                last_was_7e = *byte == 0x7e;
            }

            out.write_all(formatted.as_slice()).unwrap();
        }
        out.flush().unwrap();
    }
}

fn peek_frames(mut conn: Box<dyn physical::Connection>) {
    let mut buffer = [0u8; 1024];

    struct Sink;
    impl taptap::gateway::link::Sink for Sink {
        fn frame(&mut self, frame: Frame) {
            println!("{:?}", frame);
        }
    }

    let mut rx = taptap::gateway::link::Receiver::new(Sink);

    loop {
        let slice = match conn.read(&mut buffer) {
            Ok(n) => &buffer[0..n],
            Err(e) => {
                log::error!("error reading: {}", e);
                exit(1);
            }
        };

        if slice.is_empty() {
            return;
        }

        rx.extend_from_slice(slice);
    }
}

fn peek_activity(mut conn: Box<dyn physical::Connection>) {
    #[derive(Default)]
    struct Sink {
        slot_counters: BTreeMap<GatewayID, SlotCounter>,
    }
    impl gateway::transport::Sink for Sink {
        fn enumeration_started(&mut self, enumeration_gateway_id: GatewayID) {
            log::info!("enumeration started (at {:?})", enumeration_gateway_id);
        }

        fn gateway_identity_observed(&mut self, gateway_id: GatewayID, address: LongAddress) {
            log::info!(
                "gateway identity observed: {:?} = {:?}",
                gateway_id,
                address
            );
        }

        fn gateway_version_observed(&mut self, gateway_id: GatewayID, version: &str) {
            log::info!("gateway version observed: {:?} = {:?}", gateway_id, version);
        }

        fn enumeration_ended(&mut self, gateway_id: GatewayID) {
            log::info!("enumeration ended: {:?}", gateway_id);
        }

        fn gateway_slot_counter_captured(&mut self, _gateway_id: GatewayID) {}

        fn gateway_slot_counter_observed(
            &mut self,
            gateway_id: GatewayID,
            slot_counter: SlotCounter,
        ) {
            let print = match self.slot_counters.entry(gateway_id) {
                Entry::Vacant(e) => {
                    e.insert(slot_counter);
                    true
                }
                Entry::Occupied(mut e) => {
                    let last = e.get();
                    let print = last.epoch() != slot_counter.epoch()
                        || (last.0.get() & 0x3fff) / 1000 != (slot_counter.0.get() & 0x3fff) / 1000;
                    e.insert(slot_counter);
                    print
                }
            };

            if print {
                log::info!("slot counter: {:?} {:?}", gateway_id, slot_counter)
            }
        }

        fn packet_received(
            &mut self,
            gateway_id: GatewayID,
            header: &ReceivedPacketHeader,
            data: &[u8],
        ) {
            match header.packet_type {
                PacketType::STRING_RESPONSE
                | PacketType::POWER_REPORT
                | PacketType::TOPOLOGY_REPORT => return,
                _ => {}
            }
            log::info!("packet received: {:?} {:?} {:?}", gateway_id, header, data);
        }

        fn command_executed(
            &mut self,
            gateway_id: GatewayID,
            request: (PacketType, &[u8]),
            response: (PacketType, &[u8]),
        ) {
            match request.0 {
                PacketType::STRING_REQUEST => return,
                PacketType::NODE_TABLE_REQUEST => return,
                _ => {}
            }

            log::info!(
                "command executed: {:?} {:?} {:?} => {:?} {:?}",
                gateway_id,
                request.0,
                request.1,
                response.0,
                response.1
            );
        }
    }
    impl pv::application::Sink for Sink {
        fn string_request(&mut self, gateway_id: GatewayID, pv_node_id: NodeID, request: &str) {
            log::info!(
                "string request: {:?} {:?} {:?}",
                gateway_id,
                pv_node_id,
                request
            );
        }

        fn string_response(&mut self, gateway_id: GatewayID, pv_node_id: NodeID, response: &str) {
            log::info!(
                "string response: {:?} {:?} {:?}",
                gateway_id,
                pv_node_id,
                response
            );
        }

        fn node_table_page(
            &mut self,
            gateway_id: GatewayID,
            start_address: NodeAddress,
            nodes: &[NodeTableResponseEntry],
        ) {
            log::info!(
                "node table page: {:?} start {:?} {:?}",
                gateway_id,
                start_address,
                nodes
            );
        }

        fn topology_report(
            &mut self,
            gateway_id: GatewayID,
            pv_node_id: NodeID,
            topology_report: &TopologyReport,
        ) {
            log::info!(
                "topology report: {:?} {:?} {:?}",
                gateway_id,
                pv_node_id,
                topology_report
            );
        }

        fn power_report(
            &mut self,
            gateway_id: GatewayID,
            pv_node_id: NodeID,
            power_report: &PowerReport,
        ) {
            log::info!(
                "power report: {:?} {:?} {:?}",
                gateway_id,
                pv_node_id,
                power_report
            );
        }
    }

    let mut rx = gateway::link::Receiver::new(gateway::transport::Receiver::new(
        pv::application::Receiver::new(Sink::default()),
    ));

    let mut buffer = [0u8; 1024];
    loop {
        let slice = match conn.read(&mut buffer) {
            Ok(n) => &buffer[0..n],
            Err(e) => {
                log::error!("error reading: {}", e);
                exit(1);
            }
        };

        if slice.is_empty() {
            return;
        }

        rx.extend_from_slice(slice);
    }
}
#[cfg(feature = "serialport")]
fn list_serial_ports() {
    use serialport::SerialPortType;

    let mut ports = match physical::serialport::PortInfo::list() {
        Ok(ports) => ports,
        Err(e) => {
            log::error!("error listing serial ports: {}", e);
            exit(1);
        }
    };

    ports.sort_by_cached_key(|port| port.name().to_owned());

    if ports.is_empty() {
        println!("No serial ports detected.")
    } else {
        println!("Detected:");
    }

    for port in ports {
        println!("    --serial {}", port.name());
        match port.port_type() {
            SerialPortType::UsbPort(usb) if usb.manufacturer.is_some() && usb.product.is_some() => {
                println!(
                    "      USB {:04x}:{:04x} ({} {})",
                    usb.pid,
                    usb.vid,
                    usb.manufacturer.as_ref().unwrap(),
                    usb.product.as_ref().unwrap()
                );
            }
            SerialPortType::UsbPort(usb) => {
                println!("      USB {:04x}:{:04x}", usb.pid, usb.vid);
            }
            SerialPortType::BluetoothPort => {
                println!("      Bluetooth");
            }
            _ => {}
        }
    }
}

fn observe(mut conn: Box<dyn Connection>) {
    let observer = taptap::observer::Observer::default();
    let mut rx = gateway::link::Receiver::new(gateway::transport::Receiver::new(
        pv::application::Receiver::new(observer),
    ));

    let mut buffer = [0u8; 1024];
    loop {
        let slice = match conn.read(&mut buffer) {
            Ok(n) => &buffer[0..n],
            Err(e) => {
                log::error!("error reading: {}", e);
                exit(1);
            }
        };

        if slice.is_empty() {
            return;
        }

        rx.extend_from_slice(slice);
    }
}
