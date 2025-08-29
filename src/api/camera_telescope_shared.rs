use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// The direction in which the guide-rate motion is to be made.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Serialize_repr,
    Deserialize_repr,
    TryFromPrimitive,
    IntoPrimitive,
)]
#[repr(i32)]
pub enum GuideDirection {
    /// North (+ declination/altitude).
    North = 0,

    /// South (- declination/altitude).
    South = 1,

    /// East (+ right ascension/azimuth).
    East = 2,

    /// West (- right ascension/azimuth).
    West = 3,
}
