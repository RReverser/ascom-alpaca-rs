use std::time::SystemTime;

#[cfg(feature = "server")]
pub(crate) mod ser;

#[cfg(feature = "client")]
pub(crate) mod de;

/// A wrapper for the device-specific state with the common optional timestamp field.
#[derive(Default, Debug, Clone, derive_more::Deref, derive_more::DerefMut)]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
#[cfg_attr(feature = "client", derive(serde::Deserialize))]
pub struct TimestampedDeviceState<T> {
    /// The timestamp of the last update to the state, if available.
    #[serde(
        rename = "TimeStamp",
        skip_serializing_if = "Option::is_none",
        with = "timestamp"
    )]
    pub timestamp: Option<SystemTime>,
    /// The device-specific state.
    #[deref]
    #[deref_mut]
    #[serde(flatten)]
    pub state: T,
}

impl<T> TimestampedDeviceState<T> {
    /// Create a new `TimestampedDeviceState` with the given state and the current timestamp.
    pub fn new(state: T) -> Self {
        Self {
            timestamp: Some(SystemTime::now()),
            state,
        }
    }
}

mod timestamp {
    use crate::api::time_repr::{Iso8601, TimeRepr};
    use std::time::SystemTime;

    #[cfg(feature = "server")]
    use serde::{Serialize, Serializer};

    #[cfg(feature = "client")]
    use serde::{Deserialize, Deserializer};

    #[cfg(feature = "server")]
    #[allow(clippy::ref_option)]
    pub(super) fn serialize<S: Serializer>(
        timestamp: &Option<SystemTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        TimeRepr::<Iso8601>::from(timestamp.unwrap_or_else(|| unreachable!())).serialize(serializer)
    }

    #[cfg(feature = "client")]
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<SystemTime>, D::Error> {
        let timestamp = Option::<TimeRepr<Iso8601>>::deserialize(deserializer)?;
        Ok(timestamp.map(SystemTime::from))
    }
}
