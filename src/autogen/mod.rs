// This file is auto-generated. Do not edit it directly.

/*!
ASCOM Alpaca Device API v1

The Alpaca API uses RESTful techniques and TCP/IP to enable ASCOM applications and devices to communicate across modern network environments.

## Interface Behaviour
The ASCOM Interface behavioural requirements for Alpaca drivers are the same as for COM based drivers and are documented in the <a href="https://ascom-standards.org/Help/Developer/html/N_ASCOM_DeviceInterface.htm">API Interface Definitions</a> e.g. the <a href="https://ascom-standards.org/Help/Developer/html/M_ASCOM_DeviceInterface_ITelescopeV3_SlewToCoordinates.htm">Telescope.SlewToCoordinates</a> method. This document focuses on how to use the ASCOM Interface standards in their RESTful Alpaca form.
## Alpaca URLs, Case Sensitivity, Parameters and Returned values
**Alpaca Device API URLs** are of the form **http(s)://host:port/path** where path comprises **"/api/v1/"** followed by one of the method names below. e.g. for an Alpaca interface running on port 7843 of a device with IP address 192.168.1.89:
* A telescope "Interface Version" method URL would be **http://192.168.1.89:7843/api/v1/telescope/0/interfaceversion**
* A first focuser "Position" method URL would be  **http://192.168.1.89:7843/api/v1/focuser/0/position**
* A second focuser "StepSize" method URL would be  **http://192.168.1.89:7843/api/v1/focuser/1/stepsize**
* A rotator "Halt" method URL would be  **http://192.168.1.89:7843/api/v1/rotator/0/halt**


URLs are case sensitive and all elements must be in lower case. This means that both the device type and command name must always be in lower case. Parameter names are not case sensitive, so clients and drivers should be prepared for parameter names to be supplied and returned with any casing. Parameter values can be in mixed case as required.

For GET operations, parameters should be placed in the URL query string and for PUT operations they should be placed in the body of the message.

Responses, as described below, are returned in JSON format and always include a common set of values including the client's transaction number, the server's transaction number together with any error message and error number.
If the transaction completes successfully, the ErrorMessage field will be an empty string and the ErrorNumber field will be zero.

## HTTP Status Codes and ASCOM Error codes
The returned HTTP status code gives a high level view of whether the device understood the request and whether it attempted to process it.

Under most circumstances the returned status will be `200`, indicating that the request was correctly formatted and that it was passed to the device's handler to execute. A `200` status does not necessarily mean that the operation completed as expected, without error, and you must always check the ErrorMessage and ErrorNumber fields to confirm whether the returned result is valid. The `200` status simply means that the transaction was successfully managed by the device's transaction management layer.

An HTTP status code of `400` indicates that the device could not interpret the request e.g. an invalid device number or misspelt device type was supplied. Check the body of the response for a text error message.

An HTTP status code of `500` indicates an unexpected error within the device from which it could not recover. Check the body of the response for a text error message.
## SetupDialog and Alpaca Device Configuration
The SetupDialog method has been omitted from the Alpaca Device API because it presents a user interface rather than returning data. Alpaca device configuration is covered in the "ASCOM Alpaca Management API" specification, which can be selected through the drop-down box at the head of this page.

*/

#![allow(
  rustdoc::broken_intra_doc_links,
  clippy::doc_markdown,
  clippy::as_conversions, // triggers on derive-generated code https://github.com/rust-lang/rust-clippy/issues/9657
)]

mod server_info;

use crate::params::ascom_enum;
use crate::rpc::rpc;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
pub use server_info::*;

/// Returned camera state
#[cfg(feature = "camera")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum CameraStateResponse {
    Idle = 0,

    Waiting = 1,

    Exposing = 2,

    Reading = 3,

    Download = 4,

    Error = 5,
}

#[cfg(feature = "camera")]
ascom_enum!(CameraStateResponse);

#[cfg(feature = "camera")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum ImageArrayResponseType {
    Unknown = 0,

    /// int16
    Short = 1,

    /// int32
    Integer = 2,

    /// Double precision real number
    Double = 3,
}

#[cfg(feature = "camera")]
ascom_enum!(ImageArrayResponseType);

#[cfg(feature = "camera")]
#[path = "image_array_response.rs"]
mod image_array_response;

#[cfg(feature = "camera")]
pub use image_array_response::*;

/// Returned sensor type
#[cfg(feature = "camera")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum SensorTypeResponse {
    /// Camera produces monochrome array with no Bayer encoding
    Monochrome = 0,

    /// Camera produces color image directly, not requiring Bayer decoding
    Color = 1,

    /// Camera produces RGGB encoded Bayer array images
    RGGB = 2,

    /// Camera produces CMYG encoded Bayer array images
    CMYG = 3,

    /// Camera produces CMYG2 encoded Bayer array images
    CMYG2 = 4,

    /// Camera produces Kodak TRUESENSE LRGB encoded Bayer array images
    LRGB = 5,
}

#[cfg(feature = "camera")]
ascom_enum!(SensorTypeResponse);

/// The direction in which the guide-rate motion is to be made.
#[cfg(feature = "camera")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum PutPulseGuideDirection {
    North = 0,

    South = 1,

    East = 2,

    West = 3,
}

#[cfg(feature = "camera")]
ascom_enum!(PutPulseGuideDirection);

/// Returned side of pier
#[cfg(feature = "covercalibrator")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum CalibratorStatusResponse {
    /// This device does not have a calibration capability.
    NotPresent = 0,

    /// The calibrator is off.
    Off = 1,

    /// The calibrator is stabilising or is not yet in the commanded state.
    NotReady = 2,

    /// The calibrator is ready for use.
    Ready = 3,

    /// The calibrator state is unknown.
    Unknown = 4,

    /// The calibrator encountered an error when changing state.
    Error = 5,
}

#[cfg(feature = "covercalibrator")]
ascom_enum!(CalibratorStatusResponse);

/// Returned side of pier
#[cfg(feature = "covercalibrator")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum CoverStatusResponse {
    /// This device does not have a cover that can be closed independently.
    NotPresent = 0,

    /// The cover is closed.
    Closed = 1,

    /// The cover is moving to a new position.
    Moving = 2,

    /// The cover is open.
    Open = 3,

    /// The state of the cover is unknown.
    Unknown = 4,

    /// The device encountered an error when changing state.
    Error = 5,
}

#[cfg(feature = "covercalibrator")]
ascom_enum!(CoverStatusResponse);

/// Returned dome shutter status
#[cfg(feature = "dome")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum DomeShutterStatusResponse {
    Open = 0,

    Closed = 1,

    Opening = 2,

    Closing = 3,

    Error = 4,
}

#[cfg(feature = "dome")]
ascom_enum!(DomeShutterStatusResponse);

/// Returned side of pier
#[cfg(feature = "telescope")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum AlignmentModeResponse {
    /// Altitude-Azimuth alignment.
    AltAz = 0,

    /// Polar (equatorial) mount other than German equatorial.
    Polar = 1,

    /// German equatorial mount.
    GermanPolar = 2,
}

#[cfg(feature = "telescope")]
ascom_enum!(AlignmentModeResponse);

/// Returned side of pier
#[cfg(feature = "telescope")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum EquatorialSystemResponse {
    /// Custom or unknown equinox and/or reference frame.
    Other = 0,

    /// Topocentric coordinates. Coordinates of the object at the current date having allowed for annual aberration, precession and nutation. This is the most common coordinate type for amateur telescopes.
    Topocentric = 1,

    /// J2000 equator/equinox. Coordinates of the object at mid-day on 1st January 2000, ICRS reference frame.
    J2000 = 2,

    /// J2050 equator/equinox, ICRS reference frame.
    J2050 = 3,

    /// B1950 equinox, FK4 reference frame.
    B1950 = 4,
}

#[cfg(feature = "telescope")]
ascom_enum!(EquatorialSystemResponse);

/// Returned side of pier
#[cfg(feature = "telescope")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum SideOfPierResponse {
    /// Normal pointing state - Mount on the East side of pier (looking West).
    East = 0,

    /// Through the pole pointing state - Mount on the West side of pier (looking East).
    West = 1,

    /// Unknown or indeterminate.
    Unknown = -1,
}

#[cfg(feature = "telescope")]
ascom_enum!(SideOfPierResponse);

/// New pointing state.
#[cfg(feature = "telescope")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum TelescopeSetSideOfPierRequestSideOfPier {
    /// Normal pointing state - Mount on the East side of pier (looking West).
    East = 0,

    /// Through the pole pointing state - Mount on the West side of pier (looking East).
    West = 1,
}

#[cfg(feature = "telescope")]
ascom_enum!(TelescopeSetSideOfPierRequestSideOfPier);

/// DriveRate enum corresponding to one of the standard drive rates.
#[cfg(feature = "telescope")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum DriveRate {
    /// 15.041 arcseconds per second
    Sidereal = 0,

    /// 14.685 arcseconds per second
    Lunar = 1,

    /// 15.0 arcseconds per second
    Solar = 2,

    /// 15.0369 arcseconds per second
    King = 3,
}

#[cfg(feature = "telescope")]
ascom_enum!(DriveRate);

/// The axis of mount rotation.
#[cfg(feature = "telescope")]
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
#[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
pub enum Axis {
    Primary = 0,

    Secondary = 1,

    Tertiary = 2,
}

#[cfg(feature = "telescope")]
ascom_enum!(Axis);

/// Axis rate object
#[cfg(feature = "telescope")]
#[allow(missing_copy_implementations)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AxisRate {
    /// The maximum rate (degrees per second) This must always be a positive number. It indicates the maximum rate in either direction about the axis.
    pub maximum: f64,

    /// The minimum rate (degrees per second) This must always be a positive number. It indicates the maximum rate in either direction about the axis.
    pub minimum: f64,
}

rpc! {

    /// ASCOM Methods Common To All Devices
    pub trait Device {
        /**
        Actions and SupportedActions are a standardised means for drivers to extend functionality beyond the built-in capabilities of the ASCOM device interfaces.

        The key advantage of using Actions is that drivers can expose any device specific functionality required. The downside is that, in order to use these unique features, every application author would need to create bespoke code to present or exploit them.

        The Action parameter and return strings are deceptively simple, but can support transmission of arbitrarily complex data structures, for example through JSON encoding.

        This capability will be of primary value to
         * <span style="font-size:14px;">bespoke software and hardware configurations where a single entity controls both the consuming application software and the hardware / driver environment</span>
         * <span style="font-size:14px;">a group of application and device authors to quickly formulate and try out new interface capabilities without requiring an immediate change to the ASCOM device interface, which will take a lot longer than just agreeing a name, input parameters and a standard response for an Action command.</span>


        The list of Action commands supported by a driver can be discovered through the SupportedActions property.

        This method should return an error message and NotImplementedException error number (0x400) if the driver just implements the standard ASCOM device methods and has no bespoke, unique, functionality.
        */
        #[http("action")]
        fn action(
            &mut self,
            #[http("Action")] action: String,
            #[http("Parameters")] parameters: String,
        ) -> String;

        /// Transmits an arbitrary string to the device and does not wait for a response. Optionally, protocol framing characters may be added to the string before transmission.
        #[http("commandblind")]
        fn command_blind(&mut self, #[http("Command")] command: String, #[http("Raw")] raw: String);

        /// Transmits an arbitrary string to the device and waits for a boolean response. Optionally, protocol framing characters may be added to the string before transmission.
        #[http("commandbool")]
        fn command_bool(
            &mut self,
            #[http("Command")] command: String,
            #[http("Raw")] raw: String,
        ) -> bool;

        /// Transmits an arbitrary string to the device and waits for a string response. Optionally, protocol framing characters may be added to the string before transmission.
        #[http("commandstring")]
        fn command_string(
            &mut self,
            #[http("Command")] command: String,
            #[http("Raw")] raw: String,
        ) -> String;

        /// Retrieves the connected state of the device
        #[http("connected")]
        fn connected(&self) -> bool;

        /// Sets the connected state of the device
        #[http("connected")]
        fn set_connected(&mut self, #[http("Connected")] connected: bool);

        /// The description of the device
        #[http("description")]
        fn description(&self) -> String;

        /// The description of the driver
        #[http("driverinfo")]
        fn driver_info(&self) -> String;

        /// A string containing only the major and minor version of the driver.
        #[http("driverversion")]
        fn driver_version(&self) -> String;

        /// This method returns the version of the ASCOM device interface contract to which this device complies. Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support previous versions as well as the current version to ensure thay can use the largest number of devices.
        #[http("interfaceversion")]
        fn interface_version(&self) -> i32;

        /// The name of the device
        #[http("name")]
        fn name(&self) -> String;

        /// Returns the list of action names supported by this driver.
        #[http("supportedactions")]
        fn supported_actions(&self) -> Vec<String>;
    }

    /// Camera Specific Methods
    #[http("camera")]
    pub trait Camera {
        /// Returns the X offset of the Bayer matrix, as defined in SensorType.
        #[http("bayeroffsetx")]
        fn bayer_offset_x(&self) -> i32;

        /// Returns the Y offset of the Bayer matrix, as defined in SensorType.
        #[http("bayeroffsety")]
        fn bayer_offset_y(&self) -> i32;

        /// Returns the binning factor for the X axis.
        #[http("binx")]
        fn bin_x(&self) -> i32;

        /// Sets the binning factor for the X axis.
        #[http("binx")]
        fn set_bin_x(&mut self, #[http("BinX")] bin_x: i32);

        /// Returns the binning factor for the Y axis.
        #[http("biny")]
        fn bin_y(&self) -> i32;

        /// Sets the binning factor for the Y axis.
        #[http("biny")]
        fn set_bin_y(&mut self, #[http("BinY")] bin_y: i32);

        /// Returns the current camera operational state.
        #[http("camerastate")]
        fn camera_state(&self) -> CameraStateResponse;

        /// Returns the width of the CCD camera chip in unbinned pixels.
        #[http("cameraxsize")]
        fn camera_xsize(&self) -> i32;

        /// Returns the height of the CCD camera chip in unbinned pixels.
        #[http("cameraysize")]
        fn camera_ysize(&self) -> i32;

        /// Returns true if the camera can abort exposures; false if not.
        #[http("canabortexposure")]
        fn can_abort_exposure(&self) -> bool;

        /// Returns a flag showing whether this camera supports asymmetric binning
        #[http("canasymmetricbin")]
        fn can_asymmetric_bin(&self) -> bool;

        /// Indicates whether the camera has a fast readout mode.
        #[http("canfastreadout")]
        fn can_fast_readout(&self) -> bool;

        /// If true, the camera's cooler power setting can be read.
        #[http("cangetcoolerpower")]
        fn can_get_cooler_power(&self) -> bool;

        /// Returns a flag indicating whether this camera supports pulse guiding.
        #[http("canpulseguide")]
        fn can_pulse_guide(&self) -> bool;

        /// Returns a flag indicatig whether this camera supports setting the CCD temperature
        #[http("cansetccdtemperature")]
        fn can_set_ccdtemperature(&self) -> bool;

        /// Returns a flag indicating whether this camera can stop an exposure that is in progress
        #[http("canstopexposure")]
        fn can_stop_exposure(&self) -> bool;

        /// Returns the current CCD temperature in degrees Celsius.
        #[http("ccdtemperature")]
        fn ccdtemperature(&self) -> f64;

        /// Returns the current cooler on/off state.
        #[http("cooleron")]
        fn cooler_on(&self) -> bool;

        /// Turns on and off the camera cooler. True = cooler on, False = cooler off
        #[http("cooleron")]
        fn set_cooler_on(&mut self, #[http("CoolerOn")] cooler_on: bool);

        /// Returns the present cooler power level, in percent.
        #[http("coolerpower")]
        fn cooler_power(&self) -> f64;

        /// Returns the gain of the camera in photoelectrons per A/D unit.
        #[http("electronsperadu")]
        fn electrons_per_adu(&self) -> f64;

        /// Returns the maximum exposure time supported by StartExposure.
        #[http("exposuremax")]
        fn exposure_max(&self) -> f64;

        /// Returns the Minimium exposure time in seconds that the camera supports through StartExposure.
        #[http("exposuremin")]
        fn exposure_min(&self) -> f64;

        /// Returns the smallest increment in exposure time supported by StartExposure.
        #[http("exposureresolution")]
        fn exposure_resolution(&self) -> f64;

        /// Returns whenther Fast Readout Mode is enabled.
        #[http("fastreadout")]
        fn fast_readout(&self) -> bool;

        /// Sets whether Fast Readout Mode is enabled.
        #[http("fastreadout")]
        fn set_fast_readout(&mut self, #[http("FastReadout")] fast_readout: bool);

        /// Reports the full well capacity of the camera in electrons, at the current camera settings (binning, SetupDialog settings, etc.).
        #[http("fullwellcapacity")]
        fn full_well_capacity(&self) -> f64;

        /// The camera's gain (GAIN VALUE MODE) OR the index of the selected camera gain description in the Gains array (GAINS INDEX MODE).
        #[http("gain")]
        fn gain(&self) -> i32;

        /// The camera's gain (GAIN VALUE MODE) OR the index of the selected camera gain description in the Gains array (GAINS INDEX MODE).
        #[http("gain")]
        fn set_gain(&mut self, #[http("Gain")] gain: i32);

        /// Returns the maximum value of Gain.
        #[http("gainmax")]
        fn gain_max(&self) -> i32;

        /// Returns the Minimum value of Gain.
        #[http("gainmin")]
        fn gain_min(&self) -> i32;

        /// Returns the Gains supported by the camera.
        #[http("gains")]
        fn gains(&self) -> Vec<String>;

        /// Returns a flag indicating whether this camera has a mechanical shutter.
        #[http("hasshutter")]
        fn has_shutter(&self) -> bool;

        /// Returns the current heat sink temperature (called "ambient temperature" by some manufacturers) in degrees Celsius.
        #[http("heatsinktemperature")]
        fn heat_sink_temperature(&self) -> f64;

        /**
        Returns an array of 32bit integers containing the pixel values from the last exposure. This call can return either a 2 dimension (monochrome images) or 3 dimension (colour or multi-plane images) array of size NumX \* NumY or NumX \* NumY \* NumPlanes. Where applicable, the size of NumPlanes has to be determined by inspection of the returned Array.

        Since 32bit integers are always returned by this call, the returned JSON Type value (0 = Unknown, 1 = short(16bit), 2 = int(32bit), 3 = Double) is always 2. The number of planes is given in the returned Rank value.

        When de-serialising to an object it is essential to know the array Rank beforehand so that the correct data class can be used. This can be achieved through a regular expression or by direct parsing of the returned JSON string to extract the Type and Rank values before de-serialising.

        This regular expression accomplishes the extraction into two named groups Type and Rank, which can then be used to select the correct de-serialisation data class:

        __`^*"Type":(?<Type>\d*),"Rank":(?<Rank>\d*)`__

        When the SensorType is Monochrome, RGGB, CMYG, CMYG2 or LRGB, the serialised JSON array should have 2 dimensions. For example, the returned array should appear as below if NumX = 7, NumY = 5 and Pxy represents the pixel value at the zero based position x across and y down the image with the origin in the top left corner of the image.

        Please note that this is "column-major" order (column changes most rapidly) from the image's row and column perspective, while, from the array's perspective, serialisation is actually effected in "row-major" order (rightmost index changes most rapidly). This unintuitive outcome arises because the ASCOM Camera Interface specification defines the image column dimension as the rightmost array dimension.

        [

        [P00,P01,P02,P03,P04],

        [P10,P11,P12,P13,P14],

        [P20,P21,P22,P23,P24],

        [P30,P31,P32,P33,P34],

        [P40,P41,P42,P43,P44],

        [P50,P51,P52,P53,P54],

        [P60,P61,P62,P63,P64]

        ]

        When the SensorType is Color, the serialised JSON array will have 3 dimensions. For example, the returned array should appear as below if NumX = 7, NumY = 5 and Rxy, Gxy and Bxy represent the red, green and blue pixel values at the zero based position x across and y down the image with the origin in the top left corner of the image.  Please see note above regarding element ordering.

        [

        [[R00,G00,B00],[R01,G01,B01],[R02,G02,B02],[R03,G03,B03],[R04,G04,B04]],

        [[R10,G10,B10],[R11,G11,B11],[R12,G12,B12],[R13,G13,B13],[R14,G14,B14]],

        [[R20,G20,B20],[R21,G21,B21],[R22,G22,B22],[R23,G23,B23],[R24,G24,B24]],

        [[R30,G30,B30],[R31,G31,B31],[R32,G32,B32],[R33,G33,B33],[R34,G34,B34]],

        [[R40,G40,B40],[R41,G41,B41],[R42,G42,B42],[R43,G43,B43],[R44,G44,B44]],

        [[R50,G50,B50],[R51,G51,B51],[R52,G52,B52],[R53,G53,B53],[R54,G54,B54]],

        [[R60,G60,B60],[R61,G61,B61],[R62,G62,B62],[R63,G63,B63],[R64,G64,B64]],

        ]

        __`Performance`__

        Returning an image from an Alpaca device as a JSON array is very inefficient and can result in delays of 30 or more seconds while client and device process and send the huge JSON string over the network. A new, much faster mechanic called ImageBytes - [Alpaca ImageBytes Concepts and Implementation](https://www.ascom-standards.org/Developer/AlpacaImageBytes.pdf) has been developed that sends data as a binary byte stream and can offer a 10 to 20 fold reduction in transfer time. It is strongly recommended that Alpaca Cameras implement the ImageBytes mechanic as well as the JSON mechanic.
        */
        #[http("imagearray")]
        fn image_array(&self) -> ImageArrayResponse;

        /// Returns a flag indicating whether the image is ready to be downloaded from the camera.
        #[http("imageready")]
        fn image_ready(&self) -> bool;

        /// Returns a flag indicating whether the camera is currrently in a PulseGuide operation.
        #[http("ispulseguiding")]
        fn is_pulse_guiding(&self) -> bool;

        /// Reports the actual exposure duration in seconds (i.e. shutter open time).
        #[http("lastexposureduration")]
        fn last_exposure_duration(&self) -> f64;

        /// Reports the actual exposure start in the FITS-standard CCYY-MM-DDThh:mm:ss[.sss...] format.
        #[http("lastexposurestarttime")]
        fn last_exposure_start_time(&self) -> String;

        /// Reports the maximum ADU value the camera can produce.
        #[http("maxadu")]
        fn max_adu(&self) -> i32;

        /// Returns the maximum allowed binning for the X camera axis
        #[http("maxbinx")]
        fn max_bin_x(&self) -> i32;

        /// Returns the maximum allowed binning for the Y camera axis
        #[http("maxbiny")]
        fn max_bin_y(&self) -> i32;

        /// Returns the current subframe width, if binning is active, value is in binned pixels.
        #[http("numx")]
        fn num_x(&self) -> i32;

        /// Sets the current subframe width.
        #[http("numx")]
        fn set_num_x(&mut self, #[http("NumX")] num_x: i32);

        /// Returns the current subframe height, if binning is active, value is in binned pixels.
        #[http("numy")]
        fn num_y(&self) -> i32;

        /// Sets the current subframe height.
        #[http("numy")]
        fn set_num_y(&mut self, #[http("NumY")] num_y: i32);

        /// Returns the camera's offset (OFFSET VALUE MODE) OR the index of the selected camera offset description in the offsets array (OFFSETS INDEX MODE).
        #[http("offset")]
        fn offset(&self) -> i32;

        /// Sets the camera's offset (OFFSET VALUE MODE) OR the index of the selected camera offset description in the offsets array (OFFSETS INDEX MODE).
        #[http("offset")]
        fn set_offset(&mut self, #[http("Offset")] offset: i32);

        /// Returns the maximum value of offset.
        #[http("offsetmax")]
        fn offset_max(&self) -> i32;

        /// Returns the Minimum value of offset.
        #[http("offsetmin")]
        fn offset_min(&self) -> i32;

        /// Returns the offsets supported by the camera.
        #[http("offsets")]
        fn offsets(&self) -> Vec<String>;

        /// Returns the percentage of the current operation that is complete. If valid, returns an integer between 0 and 100, where 0 indicates 0% progress (function just started) and 100 indicates 100% progress (i.e. completion).
        #[http("percentcompleted")]
        fn percent_completed(&self) -> i32;

        /// Returns the width of the CCD chip pixels in microns.
        #[http("pixelsizex")]
        fn pixel_size_x(&self) -> f64;

        /// Returns the Height of the CCD chip pixels in microns.
        #[http("pixelsizey")]
        fn pixel_size_y(&self) -> f64;

        /// ReadoutMode is an index into the array ReadoutModes and returns the desired readout mode for the camera. Defaults to 0 if not set.
        #[http("readoutmode")]
        fn readout_mode(&self) -> i32;

        /// Sets the ReadoutMode as an index into the array ReadoutModes.
        #[http("readoutmode")]
        fn set_readout_mode(&mut self, #[http("ReadoutMode")] readout_mode: i32);

        /// This property provides an array of strings, each of which describes an available readout mode of the camera. At least one string must be present in the list.
        #[http("readoutmodes")]
        fn readout_modes(&self) -> Vec<String>;

        /// The name of the sensor used within the camera.
        #[http("sensorname")]
        fn sensor_name(&self) -> String;

        /// Returns a value indicating whether the sensor is monochrome, or what Bayer matrix it encodes.
        #[http("sensortype")]
        fn sensor_type(&self) -> SensorTypeResponse;

        /// Returns the current camera cooler setpoint in degrees Celsius.
        #[http("setccdtemperature")]
        fn set_ccdtemperature(&self) -> f64;

        /// Set's the camera's cooler setpoint in degrees Celsius.
        #[http("setccdtemperature")]
        fn set_set_ccdtemperature(&mut self, #[http("SetCCDTemperature")] set_ccdtemperature: f64);

        /// Sets the subframe start position for the X axis (0 based) and returns the current value. If binning is active, value is in binned pixels.
        #[http("startx")]
        fn start_x(&self) -> i32;

        /// Sets the current subframe X axis start position in binned pixels.
        #[http("startx")]
        fn set_start_x(&mut self, #[http("StartX")] start_x: i32);

        /// Sets the subframe start position for the Y axis (0 based) and returns the current value. If binning is active, value is in binned pixels.
        #[http("starty")]
        fn start_y(&self) -> i32;

        /// Sets the current subframe Y axis start position in binned pixels.
        #[http("starty")]
        fn set_start_y(&mut self, #[http("StartY")] start_y: i32);

        /// The Camera's sub exposure duration in seconds. Only available in Camera Interface Version 3 and later.
        #[http("subexposureduration")]
        fn sub_exposure_duration(&self) -> f64;

        /// Sets image sub exposure duration in seconds. Only available in Camera Interface Version 3 and later.
        #[http("subexposureduration")]
        fn set_sub_exposure_duration(
            &mut self,
            #[http("SubExposureDuration")] sub_exposure_duration: f64,
        );

        /// Aborts the current exposure, if any, and returns the camera to Idle state.
        #[http("abortexposure")]
        fn abort_exposure(&mut self);

        /// Activates the Camera's mount control sytem to instruct the mount to move in a particular direction for a given period of time
        #[http("pulseguide")]
        fn pulse_guide(
            &mut self,
            #[http("Direction")] direction: PutPulseGuideDirection,
            #[http("Duration")] duration: i32,
        );

        /// Starts an exposure. Use ImageReady to check when the exposure is complete.
        #[http("startexposure")]
        fn start_exposure(
            &mut self,
            #[http("Duration")] duration: f64,
            #[http("Light")] light: bool,
        );

        /// Stops the current exposure, if any. If an exposure is in progress, the readout process is initiated. Ignored if readout is already in process.
        #[http("stopexposure")]
        fn stop_exposure(&mut self);
    }

    /// CoverCalibrator Specific Methods
    #[http("covercalibrator")]
    pub trait CoverCalibrator {
        /// Returns the current calibrator brightness in the range 0 (completely off) to MaxBrightness (fully on)
        #[http("brightness")]
        fn brightness(&self) -> i32;

        /// Returns the state of the calibration device, if present, otherwise returns "NotPresent". The calibrator state mode is specified as an integer value from the CalibratorStatus Enum.
        #[http("calibratorstate")]
        fn calibrator_state(&self) -> CalibratorStatusResponse;

        /// Returns the state of the device cover, if present, otherwise returns "NotPresent". The cover state mode is specified as an integer value from the CoverStatus Enum.
        #[http("coverstate")]
        fn cover_state(&self) -> CoverStatusResponse;

        /// The Brightness value that makes the calibrator deliver its maximum illumination.
        #[http("maxbrightness")]
        fn max_brightness(&self) -> i32;

        /// Turns the calibrator off if the device has calibration capability.
        #[http("calibratoroff")]
        fn calibrator_off(&mut self);

        /// Turns the calibrator on at the specified brightness if the device has calibration capability.
        #[http("calibratoron")]
        fn calibrator_on(&mut self, #[http("Brightness")] brightness: i32);

        /// Initiates cover closing if a cover is present.
        #[http("closecover")]
        fn close_cover(&mut self);

        /// Stops any cover movement that may be in progress if a cover is present and cover movement can be interrupted.
        #[http("haltcover")]
        fn halt_cover(&mut self);

        /// Initiates cover opening if a cover is present.
        #[http("opencover")]
        fn open_cover(&mut self);
    }

    /// Dome Specific Methods
    #[http("dome")]
    pub trait Dome {
        /// The dome altitude (degrees, horizon zero and increasing positive to 90 zenith).
        #[http("altitude")]
        fn altitude(&self) -> f64;

        /// Indicates whether the dome is in the home position. This is normally used following a FindHome()  operation. The value is reset with any azimuth slew operation that moves the dome away from the home position. AtHome may also become true durng normal slew operations, if the dome passes through the home position and the dome controller hardware is capable of detecting that; or at the end of a slew operation if the dome comes to rest at the home position.
        #[http("athome")]
        fn at_home(&self) -> bool;

        /// True if the dome is in the programmed park position. Set only following a Park() operation and reset with any slew operation.
        #[http("atpark")]
        fn at_park(&self) -> bool;

        /// Returns the dome azimuth (degrees, North zero and increasing clockwise, i.e., 90 East, 180 South, 270 West)
        #[http("azimuth")]
        fn azimuth(&self) -> f64;

        /// True if the dome can move to the home position.
        #[http("canfindhome")]
        fn can_find_home(&self) -> bool;

        /// True if the dome is capable of programmed parking (Park() method)
        #[http("canpark")]
        fn can_park(&self) -> bool;

        /// True if driver is capable of setting the dome altitude.
        #[http("cansetaltitude")]
        fn can_set_altitude(&self) -> bool;

        /// True if driver is capable of setting the dome azimuth.
        #[http("cansetazimuth")]
        fn can_set_azimuth(&self) -> bool;

        /// True if driver is capable of setting the dome park position.
        #[http("cansetpark")]
        fn can_set_park(&self) -> bool;

        /// True if driver is capable of automatically operating shutter
        #[http("cansetshutter")]
        fn can_set_shutter(&self) -> bool;

        /// True if driver is capable of slaving to a telescope.
        #[http("canslave")]
        fn can_slave(&self) -> bool;

        /// True if driver is capable of synchronizing the dome azimuth position using the SyncToAzimuth(Double) method.
        #[http("cansyncazimuth")]
        fn can_sync_azimuth(&self) -> bool;

        /// Returns the status of the dome shutter or roll-off roof.
        #[http("shutterstatus")]
        fn shutter_status(&self) -> DomeShutterStatusResponse;

        /// True if the dome is slaved to the telescope in its hardware, else False.
        #[http("slaved")]
        fn slaved(&self) -> bool;

        /// Sets the current subframe height.
        #[http("slaved")]
        fn set_slaved(&mut self, #[http("Slaved")] slaved: bool);

        /// True if any part of the dome is currently moving, False if all dome components are steady.
        #[http("slewing")]
        fn slewing(&self) -> bool;

        /// Calling this method will immediately disable hardware slewing (Slaved will become False).
        #[http("abortslew")]
        fn abort_slew(&mut self);

        /// Close the shutter or otherwise shield telescope from the sky.
        #[http("closeshutter")]
        fn close_shutter(&mut self);

        /// After Home position is established initializes Azimuth to the default value and sets the AtHome flag.
        #[http("findhome")]
        fn find_home(&mut self);

        /// Open shutter or otherwise expose telescope to the sky.
        #[http("openshutter")]
        fn open_shutter(&mut self);

        /// After assuming programmed park position, sets AtPark flag.
        #[http("park")]
        fn park(&mut self);

        /// Set the current azimuth, altitude position of dome to be the park position.
        #[http("setpark")]
        fn set_park(&mut self);

        /// Slew the dome to the given altitude position.
        #[http("slewtoaltitude")]
        fn slew_to_altitude(&mut self, #[http("Altitude")] altitude: f64);

        /// Slew the dome to the given azimuth position.
        #[http("slewtoazimuth")]
        fn slew_to_azimuth(&mut self, #[http("Azimuth")] azimuth: f64);

        /// Synchronize the current position of the dome to the given azimuth.
        #[http("synctoazimuth")]
        fn sync_to_azimuth(&mut self, #[http("Azimuth")] azimuth: f64);
    }

    /// FilterWheel Specific Methods
    #[http("filterwheel")]
    pub trait FilterWheel {
        /// An integer array of filter focus offsets.
        #[http("focusoffsets")]
        fn focus_offsets(&self) -> Vec<i32>;

        /// The names of the filters
        #[http("names")]
        fn names(&self) -> Vec<String>;

        /// Returns the current filter wheel position
        #[http("position")]
        fn position(&self) -> i32;

        /// Sets the filter wheel position
        #[http("position")]
        fn set_position(&mut self, #[http("Position")] position: i32);
    }

    /// Focuser Specific Methods
    #[http("focuser")]
    pub trait Focuser {
        /// True if the focuser is capable of absolute position; that is, being commanded to a specific step location.
        #[http("absolute")]
        fn absolute(&self) -> bool;

        /// True if the focuser is currently moving to a new position. False if the focuser is stationary.
        #[http("ismoving")]
        fn is_moving(&self) -> bool;

        /// Maximum increment size allowed by the focuser; i.e. the maximum number of steps allowed in one move operation.
        #[http("maxincrement")]
        fn max_increment(&self) -> i32;

        /// Maximum step position permitted.
        #[http("maxstep")]
        fn max_step(&self) -> i32;

        /// Current focuser position, in steps.
        #[http("position")]
        fn position(&self) -> i32;

        /// Step size (microns) for the focuser.
        #[http("stepsize")]
        fn step_size(&self) -> f64;

        /// Gets the state of temperature compensation mode (if available), else always False.
        #[http("tempcomp")]
        fn temp_comp(&self) -> bool;

        /// Sets the state of temperature compensation mode.
        #[http("tempcomp")]
        fn set_temp_comp(&mut self, #[http("TempComp")] temp_comp: bool);

        /// True if focuser has temperature compensation available.
        #[http("tempcompavailable")]
        fn temp_comp_available(&self) -> bool;

        /// Current ambient temperature as measured by the focuser.
        #[http("temperature")]
        fn temperature(&self) -> f64;

        /// Immediately stop any focuser motion due to a previous Move(Int32) method call.
        #[http("halt")]
        fn halt(&mut self);

        /// Moves the focuser by the specified amount or to the specified position depending on the value of the Absolute property.
        #[http("move")]
        fn move_(&mut self, #[http("Position")] position: i32);
    }

    /// ObservingConditions Specific Methods
    #[http("observingconditions")]
    pub trait ObservingConditions {
        /// Gets the time period over which observations will be averaged
        #[http("averageperiod")]
        fn average_period(&self) -> f64;

        /// Sets the time period over which observations will be averaged
        #[http("averageperiod")]
        fn set_average_period(&mut self, #[http("AveragePeriod")] average_period: f64);

        /// Gets the percentage of the sky obscured by cloud
        #[http("cloudcover")]
        fn cloud_cover(&self) -> f64;

        /// Gets the atmospheric dew point at the observatory reported in °C.
        #[http("dewpoint")]
        fn dew_point(&self) -> f64;

        /// Gets the atmospheric  humidity (%) at the observatory
        #[http("humidity")]
        fn humidity(&self) -> f64;

        /// Gets the atmospheric pressure in hectoPascals at the observatory's altitude - NOT reduced to sea level.
        #[http("pressure")]
        fn pressure(&self) -> f64;

        /// Gets the rain rate (mm/hour) at the observatory.
        #[http("rainrate")]
        fn rain_rate(&self) -> f64;

        /// Gets the sky brightness at the observatory (Lux)
        #[http("skybrightness")]
        fn sky_brightness(&self) -> f64;

        /// Gets the sky quality at the observatory (magnitudes per square arc second)
        #[http("skyquality")]
        fn sky_quality(&self) -> f64;

        /// Gets the sky temperature(°C) at the observatory.
        #[http("skytemperature")]
        fn sky_temperature(&self) -> f64;

        /// Gets the seeing at the observatory measured as star full width half maximum (FWHM) in arc secs.
        #[http("starfwhm")]
        fn star_fwhm(&self) -> f64;

        /// Gets the temperature(°C) at the observatory.
        #[http("temperature")]
        fn temperature(&self) -> f64;

        /// Gets the wind direction. The returned value must be between 0.0 and 360.0, interpreted according to the metereological standard, where a special value of 0.0 is returned when the wind speed is 0.0. Wind direction is measured clockwise from north, through east, where East=90.0, South=180.0, West=270.0 and North=360.0.
        #[http("winddirection")]
        fn wind_direction(&self) -> f64;

        /// Gets the peak 3 second wind gust(m/s) at the observatory over the last 2 minutes.
        #[http("windgust")]
        fn wind_gust(&self) -> f64;

        /// Gets the wind speed(m/s) at the observatory.
        #[http("windspeed")]
        fn wind_speed(&self) -> f64;

        /// Forces the driver to immediately query its attached hardware to refresh sensor values.
        #[http("refresh")]
        fn refresh(&mut self);

        /// Gets a description of the sensor with the name specified in the SensorName parameter
        #[http("sensordescription")]
        fn sensor_description(&self, #[http("SensorName")] sensor_name: String) -> String;

        /// Gets the time since the sensor specified in the SensorName parameter was last updated
        #[http("timesincelastupdate")]
        fn time_since_last_update(&self, #[http("SensorName")] sensor_name: String) -> f64;
    }

    /// Rotator Specific Methods
    #[http("rotator")]
    pub trait Rotator {
        /// True if the Rotator supports the Reverse method.
        #[http("canreverse")]
        fn can_reverse(&self) -> bool;

        /// True if the rotator is currently moving to a new position. False if the focuser is stationary.
        #[http("ismoving")]
        fn is_moving(&self) -> bool;

        /// Returns the raw mechanical position of the rotator in degrees.
        #[http("mechanicalposition")]
        fn mechanical_position(&self) -> f64;

        /// Current instantaneous Rotator position, in degrees.
        #[http("position")]
        fn position(&self) -> f64;

        /// Returns the rotator’s Reverse state.
        #[http("reverse")]
        fn reverse(&self) -> bool;

        /// Sets the rotator’s Reverse state.
        #[http("reverse")]
        fn set_reverse(&mut self, #[http("Reverse")] reverse: bool);

        /// The minimum StepSize, in degrees.
        #[http("stepsize")]
        fn step_size(&self) -> f64;

        /// The destination position angle for Move() and MoveAbsolute().
        #[http("targetposition")]
        fn target_position(&self) -> f64;

        /// Immediately stop any Rotator motion due to a previous Move or MoveAbsolute method call.
        #[http("halt")]
        fn halt(&mut self);

        /// Causes the rotator to move Position degrees relative to the current Position value.
        #[http("move")]
        fn move_(&mut self, #[http("Position")] position: f64);

        /// Causes the rotator to move the absolute position of Position degrees.
        #[http("moveabsolute")]
        fn move_absolute(&mut self, #[http("Position")] position: f64);

        /// Causes the rotator to move the mechanical position of Position degrees.
        #[http("movemechanical")]
        fn move_mechanical(&mut self, #[http("Position")] position: f64);

        /// Causes the rotator to sync to the position of Position degrees.
        #[http("sync")]
        fn sync(&mut self, #[http("Position")] position: f64);
    }

    /// SafetyMonitor Specific Methods
    #[http("safetymonitor")]
    pub trait SafetyMonitor {
        /// Indicates whether the monitored state is safe for use. True if the state is safe, False if it is unsafe.
        #[http("issafe")]
        fn is_safe(&self) -> bool;
    }

    /// Switch Specific Methods
    #[http("switch")]
    pub trait Switch {
        /// Returns the number of switch devices managed by this driver. Devices are numbered from 0 to MaxSwitch - 1
        #[http("maxswitch")]
        fn max_switch(&self) -> i32;

        /// Reports if the specified switch device can be written to, default true. This is false if the device cannot be written to, for example a limit switch or a sensor.  Devices are numbered from 0 to MaxSwitch - 1
        #[http("canwrite")]
        fn can_write(&self, #[http("Id")] id: u32) -> bool;

        /// Return the state of switch device id as a boolean.  Devices are numbered from 0 to MaxSwitch - 1
        #[http("getswitch")]
        fn get_switch(&self, #[http("Id")] id: u32) -> bool;

        /// Gets the description of the specified switch device. This is to allow a fuller description of the device to be returned, for example for a tool tip. Devices are numbered from 0 to MaxSwitch - 1
        #[http("getswitchdescription")]
        fn get_switch_description(&self, #[http("Id")] id: u32) -> String;

        /// Gets the name of the specified switch device. Devices are numbered from 0 to MaxSwitch - 1
        #[http("getswitchname")]
        fn get_switch_name(&self, #[http("Id")] id: u32) -> String;

        /// Gets the value of the specified switch device as a double. Devices are numbered from 0 to MaxSwitch - 1, The value of this switch is expected to be between MinSwitchValue and MaxSwitchValue.
        #[http("getswitchvalue")]
        fn get_switch_value(&self, #[http("Id")] id: u32) -> f64;

        /// Gets the minimum value of the specified switch device as a double. Devices are numbered from 0 to MaxSwitch - 1.
        #[http("minswitchvalue")]
        fn min_switch_value(&self, #[http("Id")] id: u32) -> f64;

        /// Gets the maximum value of the specified switch device as a double. Devices are numbered from 0 to MaxSwitch - 1.
        #[http("maxswitchvalue")]
        fn max_switch_value(&self, #[http("Id")] id: u32) -> f64;

        /// Sets a switch controller device to the specified state, true or false.
        #[http("setswitch")]
        fn set_switch(&mut self, #[http("Id")] id: u32, #[http("State")] state: bool);

        /// Sets a switch device name to the specified value.
        #[http("setswitchname")]
        fn set_switch_name(&mut self, #[http("Id")] id: u32, #[http("Name")] name: String);

        /// Sets a switch device value to the specified value.
        #[http("setswitchvalue")]
        fn set_switch_value(&mut self, #[http("Id")] id: u32, #[http("Value")] value: f64);

        /// Returns the step size that this device supports (the difference between successive values of the device). Devices are numbered from 0 to MaxSwitch - 1.
        #[http("switchstep")]
        fn switch_step(&self, #[http("Id")] id: u32) -> f64;
    }

    /// Telescope Specific Methods
    #[http("telescope")]
    pub trait Telescope {
        /// Returns the alignment mode of the mount (Alt/Az, Polar, German Polar). The alignment mode is specified as an integer value from the AlignmentModes Enum.
        #[http("alignmentmode")]
        fn alignment_mode(&self) -> AlignmentModeResponse;

        /// The altitude above the local horizon of the mount's current position (degrees, positive up)
        #[http("altitude")]
        fn altitude(&self) -> f64;

        /// The area of the telescope's aperture, taking into account any obstructions (square meters)
        #[http("aperturearea")]
        fn aperture_area(&self) -> f64;

        /// The telescope's effective aperture diameter (meters)
        #[http("aperturediameter")]
        fn aperture_diameter(&self) -> f64;

        /// True if the mount is stopped in the Home position. Set only following a FindHome()  operation, and reset with any slew operation. This property must be False if the telescope does not support homing.
        #[http("athome")]
        fn at_home(&self) -> bool;

        /// True if the telescope has been put into the parked state by the seee Park()  method. Set False by calling the Unpark() method.
        #[http("atpark")]
        fn at_park(&self) -> bool;

        /// The azimuth at the local horizon of the mount's current position (degrees, North-referenced, positive East/clockwise).
        #[http("azimuth")]
        fn azimuth(&self) -> f64;

        /// True if this telescope is capable of programmed finding its home position (FindHome()  method).
        #[http("canfindhome")]
        fn can_find_home(&self) -> bool;

        /// True if this telescope is capable of programmed parking (Park() method)
        #[http("canpark")]
        fn can_park(&self) -> bool;

        /// True if this telescope is capable of software-pulsed guiding (via the PulseGuide(GuideDirections, Int32) method)
        #[http("canpulseguide")]
        fn can_pulse_guide(&self) -> bool;

        /// True if the DeclinationRate property can be changed to provide offset tracking in the declination axis.
        #[http("cansetdeclinationrate")]
        fn can_set_declination_rate(&self) -> bool;

        /// True if the guide rate properties used for PulseGuide(GuideDirections, Int32) can ba adjusted.
        #[http("cansetguiderates")]
        fn can_set_guide_rates(&self) -> bool;

        /// True if this telescope is capable of programmed setting of its park position (SetPark() method)
        #[http("cansetpark")]
        fn can_set_park(&self) -> bool;

        /// True if the SideOfPier property can be set, meaning that the mount can be forced to flip.
        #[http("cansetpierside")]
        fn can_set_pier_side(&self) -> bool;

        /// True if the RightAscensionRate property can be changed to provide offset tracking in the right ascension axis. .
        #[http("cansetrightascensionrate")]
        fn can_set_right_ascension_rate(&self) -> bool;

        /// True if the Tracking property can be changed, turning telescope sidereal tracking on and off.
        #[http("cansettracking")]
        fn can_set_tracking(&self) -> bool;

        /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to equatorial coordinates
        #[http("canslew")]
        fn can_slew(&self) -> bool;

        /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to local horizontal coordinates
        #[http("canslewaltaz")]
        fn can_slew_alt_az(&self) -> bool;

        /// True if this telescope is capable of programmed asynchronous slewing to local horizontal coordinates
        #[http("canslewaltazasync")]
        fn can_slew_alt_az_async(&self) -> bool;

        /// True if this telescope is capable of programmed asynchronous slewing to equatorial coordinates.
        #[http("canslewasync")]
        fn can_slew_async(&self) -> bool;

        /// True if this telescope is capable of programmed synching to equatorial coordinates.
        #[http("cansync")]
        fn can_sync(&self) -> bool;

        /// True if this telescope is capable of programmed synching to local horizontal coordinates
        #[http("cansyncaltaz")]
        fn can_sync_alt_az(&self) -> bool;

        /// True if this telescope is capable of programmed unparking (UnPark() method)
        #[http("canunpark")]
        fn can_unpark(&self) -> bool;

        /// The declination (degrees) of the mount's current equatorial coordinates, in the coordinate system given by the EquatorialSystem property. Reading the property will raise an error if the value is unavailable.
        #[http("declination")]
        fn declination(&self) -> f64;

        /// The declination tracking rate (arcseconds per second, default = 0.0)
        #[http("declinationrate")]
        fn declination_rate(&self) -> f64;

        /// Sets the declination tracking rate (arcseconds per second)
        #[http("declinationrate")]
        fn set_declination_rate(&mut self, #[http("DeclinationRate")] declination_rate: f64);

        /// True if the telescope or driver applies atmospheric refraction to coordinates.
        #[http("doesrefraction")]
        fn does_refraction(&self) -> bool;

        /// Causes the rotator to move Position degrees relative to the current Position value.
        #[http("doesrefraction")]
        fn set_does_refraction(&mut self, #[http("DoesRefraction")] does_refraction: bool);

        /// Returns the current equatorial coordinate system used by this telescope (e.g. Topocentric or J2000).
        #[http("equatorialsystem")]
        fn equatorial_system(&self) -> EquatorialSystemResponse;

        /// The telescope's focal length in meters
        #[http("focallength")]
        fn focal_length(&self) -> f64;

        /// The current Declination movement rate offset for telescope guiding (degrees/sec)
        #[http("guideratedeclination")]
        fn guide_rate_declination(&self) -> f64;

        /// Sets the current Declination movement rate offset for telescope guiding (degrees/sec).
        #[http("guideratedeclination")]
        fn set_guide_rate_declination(
            &mut self,
            #[http("GuideRateDeclination")] guide_rate_declination: f64,
        );

        /// The current RightAscension movement rate offset for telescope guiding (degrees/sec)
        #[http("guideraterightascension")]
        fn guide_rate_right_ascension(&self) -> f64;

        /// Sets the current RightAscension movement rate offset for telescope guiding (degrees/sec).
        #[http("guideraterightascension")]
        fn set_guide_rate_right_ascension(
            &mut self,
            #[http("GuideRateRightAscension")] guide_rate_right_ascension: f64,
        );

        /// True if a PulseGuide(GuideDirections, Int32) command is in progress, False otherwise
        #[http("ispulseguiding")]
        fn is_pulse_guiding(&self) -> bool;

        /// The right ascension (hours) of the mount's current equatorial coordinates, in the coordinate system given by the EquatorialSystem property
        #[http("rightascension")]
        fn right_ascension(&self) -> f64;

        /// The right ascension tracking rate (arcseconds per second, default = 0.0)
        #[http("rightascensionrate")]
        fn right_ascension_rate(&self) -> f64;

        /// Sets the right ascension tracking rate (arcseconds per second)
        #[http("rightascensionrate")]
        fn set_right_ascension_rate(
            &mut self,
            #[http("RightAscensionRate")] right_ascension_rate: f64,
        );

        /// Indicates the pointing state of the mount.
        #[http("sideofpier")]
        fn side_of_pier(&self) -> SideOfPierResponse;

        /// Sets the pointing state of the mount.
        #[http("sideofpier")]
        fn set_side_of_pier(
            &mut self,
            #[http("SideOfPier")] side_of_pier: TelescopeSetSideOfPierRequestSideOfPier,
        );

        /// The local apparent sidereal time from the telescope's internal clock (hours, sidereal).
        #[http("siderealtime")]
        fn sidereal_time(&self) -> f64;

        /// The elevation above mean sea level (meters) of the site at which the telescope is located.
        #[http("siteelevation")]
        fn site_elevation(&self) -> f64;

        /// Sets the elevation above mean sea level (metres) of the site at which the telescope is located.
        #[http("siteelevation")]
        fn set_site_elevation(&mut self, #[http("SiteElevation")] site_elevation: f64);

        /// The geodetic(map) latitude (degrees, positive North, WGS84) of the site at which the telescope is located.
        #[http("sitelatitude")]
        fn site_latitude(&self) -> f64;

        /// Sets the observing site's latitude (degrees).
        #[http("sitelatitude")]
        fn set_site_latitude(&mut self, #[http("SiteLatitude")] site_latitude: f64);

        /// The longitude (degrees, positive East, WGS84) of the site at which the telescope is located.
        #[http("sitelongitude")]
        fn site_longitude(&self) -> f64;

        /// Sets the observing site's longitude (degrees, positive East, WGS84).
        #[http("sitelongitude")]
        fn set_site_longitude(&mut self, #[http("SiteLongitude")] site_longitude: f64);

        /// True if telescope is currently moving in response to one of the Slew methods or the MoveAxis(TelescopeAxes, Double) method, False at all other times.
        #[http("slewing")]
        fn slewing(&self) -> bool;

        /// Returns the post-slew settling time (sec.).
        #[http("slewsettletime")]
        fn slew_settle_time(&self) -> i32;

        /// Sets the  post-slew settling time (integer sec.).
        #[http("slewsettletime")]
        fn set_slew_settle_time(&mut self, #[http("SlewSettleTime")] slew_settle_time: i32);

        /// The declination (degrees, positive North) for the target of an equatorial slew or sync operation
        #[http("targetdeclination")]
        fn target_declination(&self) -> f64;

        /// Sets the declination (degrees, positive North) for the target of an equatorial slew or sync operation
        #[http("targetdeclination")]
        fn set_target_declination(&mut self, #[http("TargetDeclination")] target_declination: f64);

        /// The right ascension (hours) for the target of an equatorial slew or sync operation
        #[http("targetrightascension")]
        fn target_right_ascension(&self) -> f64;

        /// Sets the right ascension (hours) for the target of an equatorial slew or sync operation
        #[http("targetrightascension")]
        fn set_target_right_ascension(
            &mut self,
            #[http("TargetRightAscension")] target_right_ascension: f64,
        );

        /// Returns the state of the telescope's sidereal tracking drive.
        #[http("tracking")]
        fn tracking(&self) -> bool;

        /// Sets the state of the telescope's sidereal tracking drive.
        #[http("tracking")]
        fn set_tracking(&mut self, #[http("Tracking")] tracking: bool);

        /// The current tracking rate of the telescope's sidereal drive.
        #[http("trackingrate")]
        fn tracking_rate(&self) -> i32;

        /// Sets the tracking rate of the telescope's sidereal drive.
        #[http("trackingrate")]
        fn set_tracking_rate(&mut self, #[http("TrackingRate")] tracking_rate: DriveRate);

        /// Returns an array of supported DriveRates values that describe the permissible values of the TrackingRate property for this telescope type.
        #[http("trackingrates")]
        fn tracking_rates(&self) -> Vec<DriveRate>;

        /// The UTC date/time of the telescope's internal clock in ISO 8601 format including fractional seconds. The general format (in Microsoft custom date format style) is yyyy-MM-ddTHH:mm:ss.fffffffZ E.g. 2016-03-04T17:45:31.1234567Z or 2016-11-14T07:03:08.1234567Z Please note the compulsary trailing Z indicating the 'Zulu', UTC time zone.
        #[http("utcdate")]
        fn utcdate(&self) -> String;

        /// The UTC date/time of the telescope's internal clock in ISO 8601 format including fractional seconds. The general format (in Microsoft custom date format style) is yyyy-MM-ddTHH:mm:ss.fffffffZ E.g. 2016-03-04T17:45:31.1234567Z or 2016-11-14T07:03:08.1234567Z Please note the compulsary trailing Z indicating the 'Zulu', UTC time zone.
        #[http("utcdate")]
        fn set_utcdate(&mut self, #[http("UTCDate")] utcdate: String);

        /// Immediately Stops a slew in progress.
        #[http("abortslew")]
        fn abort_slew(&mut self);

        /// The rates at which the telescope may be moved about the specified axis by the MoveAxis(TelescopeAxes, Double) method.
        #[http("axisrates")]
        fn axis_rates(&self, #[http("Axis")] axis: Axis) -> Vec<AxisRate>;

        /// True if this telescope can move the requested axis.
        #[http("canmoveaxis")]
        fn can_move_axis(&self, #[http("Axis")] axis: Axis) -> bool;

        /// Predicts the pointing state that a German equatorial mount will be in if it slews to the given coordinates.
        #[http("destinationsideofpier")]
        fn destination_side_of_pier(
            &self,
            #[http("RightAscension")] right_ascension: f64,
            #[http("Declination")] declination: f64,
        ) -> SideOfPierResponse;

        /// Locates the telescope's "home" position (synchronous)
        #[http("findhome")]
        fn find_home(&mut self);

        /// Move the telescope in one axis at the given rate.
        #[http("moveaxis")]
        fn move_axis(&mut self, #[http("Axis")] axis: Axis, #[http("Rate")] rate: f64);

        /// Move the telescope to its park position, stop all motion (or restrict to a small safe range), and set AtPark to True. )
        #[http("park")]
        fn park(&mut self);

        /// Moves the scope in the given direction for the given interval or time at the rate given by the corresponding guide rate property
        #[http("pulseguide")]
        fn pulse_guide(
            &mut self,
            #[http("Direction")] direction: PutPulseGuideDirection,
            #[http("Duration")] duration: i32,
        );

        /// Sets the telescope's park position to be its current position.
        #[http("setpark")]
        fn set_park(&mut self);

        /// Move the telescope to the given local horizontal coordinates, return when slew is complete
        #[http("slewtoaltaz")]
        fn slew_to_alt_az(
            &mut self,
            #[http("Azimuth")] azimuth: f64,
            #[http("Altitude")] altitude: f64,
        );

        /// Move the telescope to the given local horizontal coordinates, return immediatley after the slew starts. The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
        #[http("slewtoaltazasync")]
        fn slew_to_alt_az_async(
            &mut self,
            #[http("Azimuth")] azimuth: f64,
            #[http("Altitude")] altitude: f64,
        );

        /// Move the telescope to the given equatorial coordinates, return when slew is complete
        #[http("slewtocoordinates")]
        fn slew_to_coordinates(
            &mut self,
            #[http("RightAscension")] right_ascension: f64,
            #[http("Declination")] declination: f64,
        );

        /// Move the telescope to the given equatorial coordinates, return immediatley after the slew starts. The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
        #[http("slewtocoordinatesasync")]
        fn slew_to_coordinates_async(
            &mut self,
            #[http("RightAscension")] right_ascension: f64,
            #[http("Declination")] declination: f64,
        );

        /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return when slew is complete
        #[http("slewtotarget")]
        fn slew_to_target(&mut self);

        /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return immediatley after the slew starts. The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
        #[http("slewtotargetasync")]
        fn slew_to_target_async(&mut self);

        /// Matches the scope's local horizontal coordinates to the given local horizontal coordinates.
        #[http("synctoaltaz")]
        fn sync_to_alt_az(
            &mut self,
            #[http("Azimuth")] azimuth: f64,
            #[http("Altitude")] altitude: f64,
        );

        /// Matches the scope's equatorial coordinates to the given equatorial coordinates.
        #[http("synctocoordinates")]
        fn sync_to_coordinates(
            &mut self,
            #[http("RightAscension")] right_ascension: f64,
            #[http("Declination")] declination: f64,
        );

        /// Matches the scope's equatorial coordinates to the TargetRightAscension and TargetDeclination equatorial coordinates.
        #[http("synctotarget")]
        fn sync_to_target(&mut self);

        /// Takes telescope out of the Parked state. )
        #[http("unpark")]
        fn unpark(&mut self);
    }
}
