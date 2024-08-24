//! An implementation of the gateway network.
//!
//! The gateway network consists of three layers, implemented in their own modules:
//!
//! * [`physical`]
//! * [`link`]
//! * [`network`]
//! * [`application`]

pub mod application;
pub mod link;
pub mod network;
pub mod physical;

pub use application::PacketType;
pub use link::{LongAddress, ShortAddress, SlotCounter};
pub use network::NodeID;
