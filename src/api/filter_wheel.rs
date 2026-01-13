use super::Device;
use crate::{ASCOMError, ASCOMResult};
use macro_rules_attribute::apply;

/// FilterWheel Specific Methods.
#[apply(rpc_trait)]
pub trait FilterWheel: Device + Send + Sync {
    /// An integer array of filter focus offsets.
    #[http("focusoffsets", method = Get)]
    async fn focus_offsets(&self) -> ASCOMResult<Vec<i32>>;

    /// The names of the filters.
    #[http("names", method = Get)]
    async fn names(&self) -> ASCOMResult<Vec<String>>;

    /// Returns the current filter wheel position.
    ///
    /// Note: `None` indicates that the filter wheel is currently moving (equivalent to `-1` in the ASCOM specification).
    #[http("position", method = Get, via = OptionalPosition, device_state = "Position")]
    async fn position(&self) -> ASCOMResult<Option<usize>>;

    /// Sets the filter wheel position.
    #[http("position", method = Put)]
    async fn set_position(&self, #[http("Position")] position: usize) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<u16> {
        Ok(3)
    }
}

#[derive(derive_more::From, derive_more::Into)]
pub(super) struct OptionalPosition(Option<usize>);

impl serde::Serialize for OptionalPosition {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.map_or(-1, usize::cast_signed).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for OptionalPosition {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = i64::deserialize(deserializer)?;

        Ok(Self(match value {
            -1 => None,
            _ => Some(value.try_into().map_err(|_err| {
                serde::de::Error::invalid_value(
                    serde::de::Unexpected::Signed(value),
                    &"-1 (moving) or or a non-negative filter wheel position",
                )
            })?),
        }))
    }
}
