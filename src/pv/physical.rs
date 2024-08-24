use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    FromBytes,
    IntoBytes,
    Unaligned,
    KnownLayout,
    Immutable,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[repr(transparent)]
#[serde(transparent)]
pub struct RSSI(pub u8);
