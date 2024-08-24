//! An implementation of the gateway network.
//!
//! The gateway network consists of three layers, implemented in their own modules:
//!
//! * [`physical`]
//! * [`link`]
//! * [`transport`]

pub mod physical;

pub mod link;
pub use link::{Frame, GatewayID};

pub mod transport;
