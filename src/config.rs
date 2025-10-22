use crate::gateway;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename = "snake_case")]
pub enum SourceConfig {
    #[cfg(feature = "serialport")]
    Serial(SerialSourceConfig),
    Tcp(TcpConnectionConfig),
}

impl SourceConfig {
    pub fn open(&self) -> Result<Box<dyn gateway::physical::Connection>, std::io::Error> {
        match self {
            #[cfg(feature = "serialport")]
            SourceConfig::Serial(config) => {
                let conn = gateway::physical::serialport::Port::open(&config.name)?;
                Ok(Box::new(conn))
            }
            SourceConfig::Tcp(config) => {
                let addr = (config.hostname.as_str(), config.port);
                let readonly = match config.mode {
                    ConnectionMode::ReadWrite => false,
                    ConnectionMode::ReadOnly => true,
                };

                let keepalive = TcpKeepaliveConfig {
                    idle: std::time::Duration::from_secs(config.keepalive_idle),
                    interval: std::time::Duration::from_secs(config.keepalive_interval),
                    count: config.keepalive_count,
                };

                let conn = gateway::physical::tcp::Connection::connect(addr, readonly, keepalive)?;
                Ok(Box::new(conn))
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg(feature = "serialport")]
pub struct SerialSourceConfig {
    pub name: String,
}
impl From<SerialSourceConfig> for SourceConfig {
    fn from(value: SerialSourceConfig) -> Self {
        SourceConfig::Serial(value)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TcpConnectionConfig {
    pub hostname: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub mode: ConnectionMode,
    #[serde(default = "default_keepalive_idle")]
    pub keepalive_idle: u64,
    #[serde(default = "default_keepalive_interval")]
    pub keepalive_interval: u64,
    #[serde(default = "default_keepalive_count")]
    pub keepalive_count: u32,
}
impl From<TcpConnectionConfig> for SourceConfig {
    fn from(value: TcpConnectionConfig) -> Self {
        Self::Tcp(value)
    }
}

/// Configuration options for TCP keepalive.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TcpKeepaliveConfig {
    /// Idle time before keepalive probes are sent.
    pub idle: std::time::Duration,
    /// Interval between individual keepalive probes.
    pub interval: std::time::Duration,
    /// Number of unacknowledged probes before the connection is considered dead.
    pub count: u32,
}

fn default_port() -> u16 {
    502
}

fn default_keepalive_idle() -> u64 {
    30
}

fn default_keepalive_interval() -> u64 {
    10
}

fn default_keepalive_count() -> u32 {
    5
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub enum ConnectionMode {
    #[default]
    #[serde(rename = "readonly", alias = "ro")]
    ReadOnly,
    #[serde(rename = "readwrite", alias = "rw")]
    ReadWrite,
}
