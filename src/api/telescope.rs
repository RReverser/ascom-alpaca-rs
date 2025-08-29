pub use super::camera_telescope_shared::GuideDirection;

use super::time_repr::{Iso8601, TimeRepr};
use super::Device;
use crate::{ASCOMError, ASCOMResult};
use macro_rules_attribute::apply;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::ops::RangeInclusive;
use std::time::SystemTime;

/// Telescope Specific Methods.
#[apply(rpc_trait)]
pub trait Telescope: Device + Send + Sync {
    /// Returns the alignment mode of the mount (Alt/Az, Polar, German Polar).
    #[http("alignmentmode", method = Get)]
    async fn alignment_mode(&self) -> ASCOMResult<AlignmentMode> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The altitude above the local horizon of the mount's current position (degrees, positive up).
    #[http("altitude", method = Get, device_state = Altitude)]
    async fn altitude(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The area of the telescope's aperture, taking into account any obstructions (square meters).
    #[http("aperturearea", method = Get)]
    async fn aperture_area(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The telescope's effective aperture diameter (meters).
    #[http("aperturediameter", method = Get)]
    async fn aperture_diameter(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if the mount is stopped in the Home position.
    ///
    /// Set only following a FindHome()  operation, and reset with any slew operation. This property must be False if the telescope does not support homing.
    #[http("athome", method = Get, device_state = AtHome)]
    async fn at_home(&self) -> ASCOMResult<bool>;

    /// True if the telescope has been put into the parked state by the seee Park()  method.
    ///
    /// Set False by calling the Unpark() method.
    #[http("atpark", method = Get, device_state = AtPark)]
    async fn at_park(&self) -> ASCOMResult<bool>;

    /// The azimuth at the local horizon of the mount's current position (degrees, North-referenced, positive East/clockwise).
    #[http("azimuth", method = Get, device_state = Azimuth)]
    async fn azimuth(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if this telescope is capable of programmed finding its home position (FindHome()  method).
    #[http("canfindhome", method = Get)]
    async fn can_find_home(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of programmed parking (Park() method).
    #[http("canpark", method = Get)]
    async fn can_park(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of software-pulsed guiding (via the PulseGuide(GuideDirections, Int32) method).
    #[http("canpulseguide", method = Get)]
    async fn can_pulse_guide(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if the DeclinationRate property can be changed to provide offset tracking in the declination axis.
    #[http("cansetdeclinationrate", method = Get)]
    async fn can_set_declination_rate(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if the guide rate properties used for PulseGuide(GuideDirections, Int32) can ba adjusted.
    #[http("cansetguiderates", method = Get)]
    async fn can_set_guide_rates(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of programmed setting of its park position (SetPark() method).
    #[http("cansetpark", method = Get)]
    async fn can_set_park(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if the SideOfPier property can be set, meaning that the mount can be forced to flip.
    #[http("cansetpierside", method = Get)]
    async fn can_set_pier_side(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if the RightAscensionRate property can be changed to provide offset tracking in the right ascension axis. .
    #[http("cansetrightascensionrate", method = Get)]
    async fn can_set_right_ascension_rate(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if the Tracking property can be changed, turning telescope sidereal tracking on and off.
    #[http("cansettracking", method = Get)]
    async fn can_set_tracking(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to equatorial coordinates.
    #[http("canslew", method = Get)]
    async fn can_slew(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to local horizontal coordinates.
    #[http("canslewaltaz", method = Get)]
    async fn can_slew_alt_az(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of programmed asynchronous slewing to local horizontal coordinates.
    #[http("canslewaltazasync", method = Get)]
    async fn can_slew_alt_az_async(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of programmed asynchronous slewing to equatorial coordinates.
    #[http("canslewasync", method = Get)]
    async fn can_slew_async(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of programmed synching to equatorial coordinates.
    #[http("cansync", method = Get)]
    async fn can_sync(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of programmed synching to local horizontal coordinates.
    #[http("cansyncaltaz", method = Get)]
    async fn can_sync_alt_az(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if this telescope is capable of programmed unparking (UnPark() method).
    #[http("canunpark", method = Get)]
    async fn can_unpark(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// The declination (degrees) of the mount's current equatorial coordinates, in the coordinate system given by the EquatorialSystem property.
    ///
    /// Reading the property will raise an error if the value is unavailable.
    #[http("declination", method = Get, device_state = Declination)]
    async fn declination(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The declination tracking rate (arcseconds per SI second, default = 0.0).
    ///
    /// Please note that rightascensionrate units are arcseconds per sidereal second.
    #[http("declinationrate", method = Get)]
    async fn declination_rate(&self) -> ASCOMResult<f64>;

    /// Sets the declination tracking rate (arcseconds per SI second).
    ///
    /// Please note that rightascensionrate units are arcseconds per sidereal second.
    #[http("declinationrate", method = Put)]
    async fn set_declination_rate(
        &self,

        #[http("DeclinationRate")] declination_rate: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if the telescope or driver applies atmospheric refraction to coordinates.
    #[http("doesrefraction", method = Get)]
    async fn does_refraction(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Causes the rotator to move Position degrees relative to the current Position value.
    #[http("doesrefraction", method = Put)]
    async fn set_does_refraction(
        &self,

        #[http("DoesRefraction")] does_refraction: bool,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current equatorial coordinate system used by this telescope (e.g. Topocentric or J2000).
    #[http("equatorialsystem", method = Get)]
    async fn equatorial_system(&self) -> ASCOMResult<EquatorialCoordinateType>;

    /// The telescope's focal length in meters.
    #[http("focallength", method = Get)]
    async fn focal_length(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The current Declination movement rate offset for telescope guiding (degrees/sec).
    #[http("guideratedeclination", method = Get)]
    async fn guide_rate_declination(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the current Declination movement rate offset for telescope guiding (degrees/sec).
    #[http("guideratedeclination", method = Put)]
    async fn set_guide_rate_declination(
        &self,

        #[http("GuideRateDeclination")] guide_rate_declination: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The current RightAscension movement rate offset for telescope guiding (degrees/sec).
    #[http("guideraterightascension", method = Get)]
    async fn guide_rate_right_ascension(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the current RightAscension movement rate offset for telescope guiding (degrees/sec).
    #[http("guideraterightascension", method = Put)]
    async fn set_guide_rate_right_ascension(
        &self,

        #[http("GuideRateRightAscension")] guide_rate_right_ascension: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if a PulseGuide(GuideDirections, Int32) command is in progress, False otherwise.
    #[http("ispulseguiding", method = Get, device_state = IsPulseGuiding)]
    async fn is_pulse_guiding(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The right ascension (hours) of the mount's current equatorial coordinates, in the coordinate system given by the EquatorialSystem property.
    #[http("rightascension", method = Get, device_state = RightAscension)]
    async fn right_ascension(&self) -> ASCOMResult<f64>;

    /// The right ascension tracking rate (arcseconds per sidereal second, default = 0.0).
    ///
    /// Please note that the declinationrate units are arcseconds per SI second.
    #[http("rightascensionrate", method = Get)]
    async fn right_ascension_rate(&self) -> ASCOMResult<f64>;

    /// Sets the right ascension tracking rate (arcseconds per sidereal second).
    ///
    /// Please note that the declinationrate units are arcseconds per SI second.
    #[http("rightascensionrate", method = Put)]
    async fn set_right_ascension_rate(
        &self,

        #[http("RightAscensionRate")] right_ascension_rate: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Indicates the pointing state of the mount.
    #[http("sideofpier", method = Get, device_state = SideOfPier)]
    async fn side_of_pier(&self) -> ASCOMResult<PierSide> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the pointing state of the mount.
    #[http("sideofpier", method = Put)]
    async fn set_side_of_pier(
        &self,
        #[http("SideOfPier")] side_of_pier: PierSide,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The local apparent sidereal time from the telescope's internal clock (hours, sidereal).
    #[http("siderealtime", method = Get)]
    async fn sidereal_time(&self) -> ASCOMResult<f64>;

    /// The elevation above mean sea level (meters) of the site at which the telescope is located.
    #[http("siteelevation", method = Get)]
    async fn site_elevation(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the elevation above mean sea level (metres) of the site at which the telescope is located.
    #[http("siteelevation", method = Put)]
    async fn set_site_elevation(
        &self,

        #[http("SiteElevation")] site_elevation: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The geodetic(map) latitude (degrees, positive North, WGS84) of the site at which the telescope is located.
    #[http("sitelatitude", method = Get)]
    async fn site_latitude(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the observing site's latitude (degrees).
    #[http("sitelatitude", method = Put)]
    async fn set_site_latitude(
        &self,
        #[http("SiteLatitude")] site_latitude: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The longitude (degrees, positive East, WGS84) of the site at which the telescope is located.
    #[http("sitelongitude", method = Get)]
    async fn site_longitude(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the observing site's longitude (degrees, positive East, WGS84).
    #[http("sitelongitude", method = Put)]
    async fn set_site_longitude(
        &self,

        #[http("SiteLongitude")] site_longitude: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if telescope is currently moving in response to one of the Slew methods or the MoveAxis(TelescopeAxes, Double) method, False at all other times.
    #[http("slewing", method = Get, device_state = Slewing)]
    async fn slewing(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the post-slew settling time (sec.).
    #[http("slewsettletime", method = Get)]
    async fn slew_settle_time(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the  post-slew settling time (integer sec.).
    #[http("slewsettletime", method = Put)]
    async fn set_slew_settle_time(
        &self,

        #[http("SlewSettleTime")] slew_settle_time: i32,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The declination (degrees, positive North) for the target of an equatorial slew or sync operation.
    #[http("targetdeclination", method = Get)]
    async fn target_declination(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the declination (degrees, positive North) for the target of an equatorial slew or sync operation.
    #[http("targetdeclination", method = Put)]
    async fn set_target_declination(
        &self,

        #[http("TargetDeclination")] target_declination: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The right ascension (hours) for the target of an equatorial slew or sync operation.
    #[http("targetrightascension", method = Get)]
    async fn target_right_ascension(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the right ascension (hours) for the target of an equatorial slew or sync operation.
    #[http("targetrightascension", method = Put)]
    async fn set_target_right_ascension(
        &self,

        #[http("TargetRightAscension")] target_right_ascension: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the state of the telescope's sidereal tracking drive.
    #[http("tracking", method = Get, device_state = Tracking)]
    async fn tracking(&self) -> ASCOMResult<bool>;

    /// Sets the state of the telescope's sidereal tracking drive.
    #[http("tracking", method = Put)]
    async fn set_tracking(&self, #[http("Tracking")] tracking: bool) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The current tracking rate of the telescope's sidereal drive.
    #[http("trackingrate", method = Get)]
    async fn tracking_rate(&self) -> ASCOMResult<DriveRate>;

    /// Sets the tracking rate of the telescope's sidereal drive.
    #[http("trackingrate", method = Put)]
    async fn set_tracking_rate(
        &self,

        #[http("TrackingRate")] tracking_rate: DriveRate,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns an array of supported DriveRates values that describe the permissible values of the TrackingRate property for this telescope type.
    #[http("trackingrates", method = Get)]
    async fn tracking_rates(&self) -> ASCOMResult<Vec<DriveRate>> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the UTC date/time of the telescope's internal clock.
    #[http("utcdate", method = Get, via = TimeRepr<Iso8601>)]
    async fn utc_date(&self) -> ASCOMResult<SystemTime>;

    /// Sets the UTC date/time of the telescope's internal clock.
    #[http("utcdate", method = Put)]
    async fn set_utc_date(
        &self,

        #[http("UTCDate", via = TimeRepr<Iso8601>)] utc_date: SystemTime,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Immediately Stops a slew in progress.
    #[http("abortslew", method = Put)]
    async fn abort_slew(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The rates at which the telescope may be moved about the specified axis by the MoveAxis(TelescopeAxes, Double) method.
    #[http("axisrates", method = Get, via = AxisRates)]
    async fn axis_rates(
        &self,
        #[http("Axis")] axis: TelescopeAxis,
    ) -> ASCOMResult<Vec<RangeInclusive<f64>>>;

    /// True if this telescope can move the requested axis.
    #[http("canmoveaxis", method = Get)]
    async fn can_move_axis(&self, #[http("Axis")] axis: TelescopeAxis) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Predicts the pointing state that a German equatorial mount will be in if it slews to the given coordinates.
    #[http("destinationsideofpier", method = Get)]
    async fn destination_side_of_pier(
        &self,

        #[http("RightAscension")] right_ascension: f64,

        #[http("Declination")] declination: f64,
    ) -> ASCOMResult<PierSide> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Locates the telescope's "home" position (synchronous).
    #[http("findhome", method = Put)]
    async fn find_home(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Move the telescope in one axis at the given rate.
    #[http("moveaxis", method = Put)]
    async fn move_axis(
        &self,

        #[http("Axis")] axis: TelescopeAxis,

        #[http("Rate")] rate: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Move the telescope to its park position, stop all motion (or restrict to a small safe range), and set AtPark to True. ).
    #[http("park", method = Put)]
    async fn park(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Moves the scope in the given direction for the given interval or time at the rate given by the corresponding guide rate property.
    #[http("pulseguide", method = Put)]
    async fn pulse_guide(
        &self,

        #[http("Direction")] direction: GuideDirection,

        #[http("Duration")] duration: i32,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the telescope's park position to be its current position.
    #[http("setpark", method = Put)]
    async fn set_park(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Move the telescope to the given local horizontal coordinates, return when slew is complete.
    #[http("slewtoaltaz", method = Put)]
    async fn slew_to_alt_az(
        &self,

        #[http("Azimuth")] azimuth: f64,

        #[http("Altitude")] altitude: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Move the telescope to the given local horizontal coordinates, return immediately after the slew starts.
    ///
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    #[http("slewtoaltazasync", method = Put)]
    async fn slew_to_alt_az_async(
        &self,

        #[http("Azimuth")] azimuth: f64,

        #[http("Altitude")] altitude: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Move the telescope to the given equatorial coordinates, return when slew is complete.
    #[http("slewtocoordinates", method = Put)]
    async fn slew_to_coordinates(
        &self,

        #[http("RightAscension")] right_ascension: f64,

        #[http("Declination")] declination: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Move the telescope to the given equatorial coordinates, return immediatley after the slew starts.
    ///
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    #[http("slewtocoordinatesasync", method = Put)]
    async fn slew_to_coordinates_async(
        &self,

        #[http("RightAscension")] right_ascension: f64,

        #[http("Declination")] declination: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// **This method is deprecated in favour of [`slew_to_target_async`](Self::slew_to_target_async).**
    ///
    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return when slew is complete.
    #[http("slewtotarget", method = Put)]
    async fn slew_to_target(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return immediatley after the slew starts.
    ///
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    #[http("slewtotargetasync", method = Put)]
    async fn slew_to_target_async(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Matches the scope's local horizontal coordinates to the given local horizontal coordinates.
    #[http("synctoaltaz", method = Put)]
    async fn sync_to_alt_az(
        &self,

        #[http("Azimuth")] azimuth: f64,

        #[http("Altitude")] altitude: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Matches the scope's equatorial coordinates to the given equatorial coordinates.
    #[http("synctocoordinates", method = Put)]
    async fn sync_to_coordinates(
        &self,

        #[http("RightAscension")] right_ascension: f64,

        #[http("Declination")] declination: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Matches the scope's equatorial coordinates to the TargetRightAscension and TargetDeclination equatorial coordinates.
    #[http("synctotarget", method = Put)]
    async fn sync_to_target(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Takes telescope out of the Parked state.
    #[http("unpark", method = Put)]
    async fn unpark(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(4_i32)
    }
}

/// The alignment mode (geometry) of the mount.
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
pub enum AlignmentMode {
    /// Altitude-Azimuth type mount.
    AltAz = 0,

    /// Polar (equatorial) mount other than German equatorial.
    Polar = 1,

    /// German equatorial type mount.
    GermanPolar = 2,
}

/// The equatorial coordinate system used by the mount.
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
pub enum EquatorialCoordinateType {
    /// Custom or unknown equinox and/or reference frame.
    Other = 0,

    /// Topocentric coordinates.
    Topocentric = 1,

    /// J2000 equator/equinox.
    J2000 = 2,

    /// J2050 equator/equinox.
    J2050 = 3,

    /// B1950 equinox, FK4 reference frame.
    B1950 = 4,
}

/// Returned side of pier.
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
pub enum PierSide {
    /// Normal pointing state - Mount on the East side of pier (looking West).
    East = 0,

    /// Through the pole pointing state - Mount on the West side of pier (looking East).
    West = 1,

    /// Unknown or indeterminate.
    Unknown = -1,
}

/// Integer value corresponding to one of the standard drive rates.
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
pub enum DriveRate {
    /// Sidereal tracking rate (15.041 arcseconds per second).
    Sidereal = 0,

    /// Lunar tracking rate (14.685 arcseconds per second).
    Lunar = 1,

    /// Solar tracking rate (15.0 arcseconds per second).
    Solar = 2,

    /// King tracking rate (15.0369 arcseconds per second).
    King = 3,
}

/// Axis rate object.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AxisRate {
    /// The minimum rate (degrees per second).
    ///
    /// This must always be a positive number. It indicates the maximum rate in either direction about the axis.
    pub minimum: f64,

    /// The maximum rate (degrees per second).
    ///
    /// This must always be a positive number. It indicates the maximum rate in either direction about the axis.
    pub maximum: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct AxisRates(Vec<AxisRate>);

impl From<AxisRates> for Vec<RangeInclusive<f64>> {
    fn from(axis_rates: AxisRates) -> Self {
        axis_rates
            .0
            .into_iter()
            .map(|axis_rate| axis_rate.minimum..=axis_rate.maximum)
            .collect()
    }
}

impl From<Vec<RangeInclusive<f64>>> for AxisRates {
    fn from(ranges: Vec<RangeInclusive<f64>>) -> Self {
        Self(
            ranges
                .into_iter()
                .map(|range| {
                    let (minimum, maximum) = range.into_inner();
                    AxisRate { minimum, maximum }
                })
                .collect(),
        )
    }
}

/// The axis about which rate information is desired.
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
pub enum TelescopeAxis {
    /// Primary axis (e.g., Right Ascension or Azimuth).
    Primary = 0,

    /// Secondary axis (e.g., Declination or Altitude).
    Secondary = 1,

    /// Tertiary axis (e.g. imager rotator/de-rotator).
    Tertiary = 2,
}
