mod image_array;
pub use image_array::*;

pub use super::camera_telescope_shared::GuideDirection;

use super::Device;
use super::time_repr::{Fits, TimeRepr};
use crate::{ASCOMError, ASCOMResult};
use macro_rules_attribute::apply;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde_repr::{Deserialize_repr, Serialize_repr};
#[cfg(feature = "client")]
use std::ops::RangeInclusive;
use std::time::SystemTime;

/// Camera Specific Methods.
#[apply(rpc_trait)]
pub trait Camera: Device + Send + Sync {
    /// Returns the X offset of the Bayer matrix, as defined in SensorType.
    #[http("bayeroffsetx", method = Get)]
    async fn bayer_offset_x(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the Y offset of the Bayer matrix, as defined in SensorType.
    #[http("bayeroffsety", method = Get)]
    async fn bayer_offset_y(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the binning factor for the X axis.
    #[http("binx", method = Get)]
    async fn bin_x(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the binning factor for the X axis.
    #[http("binx", method = Put)]
    async fn set_bin_x(&self, #[http("BinX")] bin_x: i32) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the binning factor for the Y axis.
    #[http("biny", method = Get)]
    async fn bin_y(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the binning factor for the Y axis.
    #[http("biny", method = Put)]
    async fn set_bin_y(&self, #[http("BinY")] bin_y: i32) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current camera operational state as an integer.
    #[http("camerastate", method = Get, device_state = "CameraState")]
    async fn camera_state(&self) -> ASCOMResult<CameraState> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the width of the CCD camera chip in unbinned pixels.
    #[http("cameraxsize", method = Get)]
    async fn camera_x_size(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the height of the CCD camera chip in unbinned pixels.
    #[http("cameraysize", method = Get)]
    async fn camera_y_size(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns true if the camera can abort exposures; false if not.
    #[http("canabortexposure", method = Get)]
    async fn can_abort_exposure(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Returns a flag showing whether this camera supports asymmetric binning.
    #[http("canasymmetricbin", method = Get)]
    async fn can_asymmetric_bin(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Indicates whether the camera has a fast readout mode.
    #[http("canfastreadout", method = Get)]
    async fn can_fast_readout(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// If true, the camera's cooler power setting can be read.
    #[http("cangetcoolerpower", method = Get)]
    async fn can_get_cooler_power(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Returns a flag indicating whether this camera supports pulse guiding.
    #[http("canpulseguide", method = Get)]
    async fn can_pulse_guide(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Returns a flag indicatig whether this camera supports setting the CCD temperature.
    #[http("cansetccdtemperature", method = Get)]
    async fn can_set_ccd_temperature(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Returns a flag indicating whether this camera can stop an exposure that is in progress.
    #[http("canstopexposure", method = Get)]
    async fn can_stop_exposure(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Returns the current CCD temperature in degrees Celsius.
    #[http("ccdtemperature", method = Get, device_state = "CCDTemperature")]
    async fn ccd_temperature(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current cooler on/off state.
    #[http("cooleron", method = Get)]
    async fn cooler_on(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Turns on and off the camera cooler.
    ///
    /// True = cooler on, False = cooler off.
    #[http("cooleron", method = Put)]
    async fn set_cooler_on(&self, #[http("CoolerOn")] cooler_on: bool) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the present cooler power level, in percent.
    #[http("coolerpower", method = Get, device_state = "CoolerPower")]
    async fn cooler_power(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the gain of the camera in photoelectrons per A/D unit.
    #[http("electronsperadu", method = Get)]
    async fn electrons_per_adu(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the maximum exposure time in seconds supported by StartExposure.
    #[http("exposuremax", method = Get)]
    async fn exposure_max(&self) -> ASCOMResult<f64>;

    /// Returns the minimium exposure time in seconds supported by StartExposure.
    #[http("exposuremin", method = Get)]
    async fn exposure_min(&self) -> ASCOMResult<f64>;

    /// Returns the smallest increment in exposure time supported by StartExposure.
    #[http("exposureresolution", method = Get)]
    async fn exposure_resolution(&self) -> ASCOMResult<f64>;

    /// Returns whenther Fast Readout Mode is enabled.
    #[http("fastreadout", method = Get)]
    async fn fast_readout(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets whether Fast Readout Mode is enabled.
    #[http("fastreadout", method = Put)]
    async fn set_fast_readout(&self, #[http("FastReadout")] fast_readout: bool) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Reports the full well capacity of the camera in electrons, at the current camera settings (binning, SetupDialog settings, etc.).
    #[http("fullwellcapacity", method = Get)]
    async fn full_well_capacity(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The camera's gain (GAIN VALUE MODE) OR the index of the selected camera gain description in the Gains array (GAINS INDEX MODE).
    #[http("gain", method = Get)]
    async fn gain(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The camera's gain (GAIN VALUE MODE) OR the index of the selected camera gain description in the Gains array (GAINS INDEX MODE).
    #[http("gain", method = Put)]
    async fn set_gain(&self, #[http("Gain")] gain: i32) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the maximum value of Gain.
    #[http("gainmax", method = Get)]
    async fn gain_max(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the minimum value of Gain.
    #[http("gainmin", method = Get)]
    async fn gain_min(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the Gains supported by the camera.
    #[http("gains", method = Get)]
    async fn gains(&self) -> ASCOMResult<Vec<String>> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns a flag indicating whether this camera has a mechanical shutter.
    #[http("hasshutter", method = Get)]
    async fn has_shutter(&self) -> ASCOMResult<bool>;

    /// Returns the current heat sink temperature (called "ambient temperature" by some manufacturers) in degrees Celsius.
    #[http("heatsinktemperature", method = Get, device_state = "HeatSinkTemperature")]
    async fn heat_sink_temperature(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns an array of 32bit integers containing the pixel values from the last exposure.
    ///
    /// This call can return either a 2 dimension (monochrome images) or 3 dimension (colour or multi-plane images) array  of size `NumX * NumY` or `NumX * NumY * NumPlanes`. Where applicable, the size of `NumPlanes` has to be determined by inspection of the returned array.
    #[http("imagearray", method = Get)]
    async fn image_array(&self) -> ASCOMResult<ImageArray> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns a flag indicating whether the image is ready to be downloaded from the camera.
    #[http("imageready", method = Get, device_state = "ImageReady")]
    async fn image_ready(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns a flag indicating whether the camera is currrently in a PulseGuide operation.
    #[http("ispulseguiding", method = Get, device_state = "IsPulseGuiding")]
    async fn is_pulse_guiding(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Reports the actual exposure duration in seconds (i.e. shutter open time).
    #[http("lastexposureduration", method = Get)]
    async fn last_exposure_duration(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Reports the actual exposure start.
    ///
    /// The time must be UTC.
    #[http("lastexposurestarttime", method = Get, via = TimeRepr<Fits>)]
    async fn last_exposure_start_time(&self) -> ASCOMResult<SystemTime> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Reports the maximum ADU value the camera can produce.
    #[http("maxadu", method = Get)]
    async fn max_adu(&self) -> ASCOMResult<i32>;

    /// Returns the maximum allowed binning for the X camera axis.
    #[http("maxbinx", method = Get)]
    async fn max_bin_x(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the maximum allowed binning for the Y camera axis.
    #[http("maxbiny", method = Get)]
    async fn max_bin_y(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current subframe width in binned pixels.
    #[http("numx", method = Get)]
    async fn num_x(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the current subframe width in binned pixels.
    #[http("numx", method = Put)]
    async fn set_num_x(&self, #[http("NumX")] num_x: i32) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current subframe height in binned pixels.
    #[http("numy", method = Get)]
    async fn num_y(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the current subframe height in binned pixels.
    #[http("numy", method = Put)]
    async fn set_num_y(&self, #[http("NumY")] num_y: i32) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the camera's offset (OFFSET VALUE MODE) OR the index of the selected camera offset description in the offsets array (OFFSETS INDEX MODE).
    #[http("offset", method = Get)]
    async fn offset(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the camera's offset (OFFSET VALUE MODE) OR the index of the selected camera offset description in the offsets array (OFFSETS INDEX MODE).
    #[http("offset", method = Put)]
    async fn set_offset(&self, #[http("Offset")] offset: i32) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the maximum value of offset.
    #[http("offsetmax", method = Get)]
    async fn offset_max(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the Minimum value of offset.
    #[http("offsetmin", method = Get)]
    async fn offset_min(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the offsets supported by the camera.
    #[http("offsets", method = Get)]
    async fn offsets(&self) -> ASCOMResult<Vec<String>> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the percentage of the current operation that is complete.
    ///
    /// If valid, returns an integer between 0 and 100, where 0 indicates 0% progress (function just started) and 100 indicates 100% progress (i.e. completion).
    #[http("percentcompleted", method = Get, device_state = "PercentCompleted")]
    async fn percent_completed(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the width of the CCD chip pixels in microns.
    #[http("pixelsizex", method = Get)]
    async fn pixel_size_x(&self) -> ASCOMResult<f64>;

    /// Returns the Height of the CCD chip pixels in microns.
    #[http("pixelsizey", method = Get)]
    async fn pixel_size_y(&self) -> ASCOMResult<f64>;

    /// ReadoutMode is an index into the array ReadoutModes and returns the desired readout mode for the camera.
    ///
    /// Defaults to 0 if not set.
    #[http("readoutmode", method = Get)]
    async fn readout_mode(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the ReadoutMode as an index into the array ReadoutModes.
    #[http("readoutmode", method = Put)]
    async fn set_readout_mode(&self, #[http("ReadoutMode")] readout_mode: i32) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// This property provides an array of strings, each of which describes an available readout mode of the camera.
    ///
    /// At least one string must be present in the list.
    #[http("readoutmodes", method = Get)]
    async fn readout_modes(&self) -> ASCOMResult<Vec<String>> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The name of the sensor used within the camera.
    #[http("sensorname", method = Get)]
    async fn sensor_name(&self) -> ASCOMResult<String> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns a value indicating whether the sensor is monochrome, or what Bayer matrix it encodes.
    #[http("sensortype", method = Get)]
    async fn sensor_type(&self) -> ASCOMResult<SensorType> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current camera cooler setpoint in degrees Celsius.
    #[http("setccdtemperature", method = Get)]
    async fn set_ccd_temperature(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Set's the camera's cooler setpoint in degrees Celsius.
    #[http("setccdtemperature", method = Put)]
    async fn set_set_ccd_temperature(
        &self,

        #[http("SetCCDTemperature")] set_ccd_temperature: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current subframe start position for the X axis (0 based) in binned pixels.
    #[http("startx", method = Get)]
    async fn start_x(&self) -> ASCOMResult<i32>;

    /// Sets the current subframe X axis start position in binned pixels.
    #[http("startx", method = Put)]
    async fn set_start_x(&self, #[http("StartX")] start_x: i32) -> ASCOMResult<()>;

    /// Returns the current subframe start position for the Y axis (0 based) in binned pixels.
    #[http("starty", method = Get)]
    async fn start_y(&self) -> ASCOMResult<i32>;

    /// Sets the current subframe Y axis start position in binned pixels.
    #[http("starty", method = Put)]
    async fn set_start_y(&self, #[http("StartY")] start_y: i32) -> ASCOMResult<()>;

    /// The Camera's sub exposure duration in seconds.
    ///
    /// _ICameraV3 and later._
    #[http("subexposureduration", method = Get)]
    async fn sub_exposure_duration(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets image sub exposure duration in seconds.
    ///
    /// _ICameraV3 and later._
    #[http("subexposureduration", method = Put)]
    async fn set_sub_exposure_duration(
        &self,

        #[http("SubExposureDuration")] sub_exposure_duration: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Aborts the current exposure, if any, and returns the camera to Idle state.
    #[http("abortexposure", method = Put)]
    async fn abort_exposure(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Activates the Camera's mount control sytem to instruct the mount to move in a particular direction for a given period of time.
    #[http("pulseguide", method = Put)]
    async fn pulse_guide(
        &self,

        #[http("Direction")] direction: GuideDirection,

        #[http("Duration")] duration: i32,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Starts an exposure.
    ///
    /// Use ImageReady to check when the exposure is complete.
    #[http("startexposure", method = Put)]
    async fn start_exposure(
        &self,

        #[http("Duration")] duration: f64,

        #[http("Light")] light: bool,
    ) -> ASCOMResult<()>;

    /// Stops the current exposure, if any.
    ///
    /// If an exposure is in progress, the readout process is initiated. Ignored if readout is already in process.
    #[http("stopexposure", method = Put)]
    async fn stop_exposure(&self) -> ASCOMResult<()> {
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

convenience_props!(Camera {
    /// Returns the X and Y offsets of the Bayer matrix, as defined in SensorType.
    bayer_offset(bayer_offset_x, bayer_offset_y): [i32; 2],

    /// Returns the binning factors for the X and Y axes.
    #[
        /// Sets the binning factors for the X and Y axes.
        set
    ]
    bin(bin_x, bin_y): [i32; 2],

    /// Returns the width and height of the CCD camera chip in unbinned pixels.
    camera_size(camera_x_size, camera_y_size): [i32; 2],

    /// Returns the exposure time range in seconds supported by StartExposure.
    exposure_range(exposure_min, exposure_max): RangeInclusive<f64>,

    /// Returns the supported gain range.
    gain_range(gain_min, gain_max): RangeInclusive<i32>,

    /// Returns the maximum allowed binning for the X and Y camera axes.
    max_bin(max_bin_x, max_bin_y): [i32; 2],

    /// Returns the current subframe width and height in binned pixels.
    #[
        /// Sets the current subframe width and height in binned pixels.
        set
    ]
    num(num_x, num_y): [i32; 2],

    /// Returns the supported offset range.
    offset_range(offset_min, offset_max): RangeInclusive<i32>,

    /// Returns the width and height of the CCD chip pixels in microns.
    pixel_size(pixel_size_x, pixel_size_y): [f64; 2],

    /// Returns the current subframe start position for the X and Y axes (0 based) in binned pixels.
    start(start_x, start_y): [i32; 2],
});

/// Camera gain mode.
///
/// See [ASCOM docs](https://ascom-standards.org/newdocs/camera.html#Camera.Gain) for more details on gain modes.
#[cfg(feature = "client")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GainMode {
    /// A range of valid Gain values.
    Range(RangeInclusive<i32>),
    /// A list of valid Gain values.
    List(Vec<String>),
}

#[cfg(feature = "client")]
impl dyn Camera {
    /// Return the camera gain mode with valid Gain values, or `None` if Gain is not supported.
    ///
    /// This is a convenience method for clients aggregating following properties:
    /// - [`gain_min`](Camera::gain_min)
    /// - [`gain_max`](Camera::gain_max)
    /// - [`gains`](Camera::gains)
    pub async fn gain_mode(&self) -> ASCOMResult<Option<GainMode>> {
        fn if_implemented<T>(res: ASCOMResult<T>) -> ASCOMResult<Option<T>> {
            match res {
                Err(err) if err.code == crate::ASCOMErrorCode::NOT_IMPLEMENTED => Ok(None),
                _ => res.map(Some),
            }
        }

        // Try to get the gain list first.
        Ok(match if_implemented(self.gains().await)? {
            Some(gains) => Some(GainMode::List(gains)),
            None => {
                // If gain list is not supported, we fall back to the gain range.
                if_implemented(self.gain_range().await)?.map(GainMode::Range)
            }
        })
    }
}

/// Camera state.
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
pub enum CameraState {
    /// At idle state, available to start exposure.
    Idle = 0,

    /// Exposure started but waiting (for shutter, trigger, filter wheel, etc.).
    Waiting = 1,

    /// Exposure currently in progress.
    Exposing = 2,

    /// Sensor array is being read out (digitized).
    Reading = 3,

    /// Downloading data to host.
    Download = 4,

    /// Camera error condition serious enough to prevent further operations.
    Error = 5,
}

/// The type of sensor in the camera.
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
pub enum SensorType {
    /// Single-plane monochrome sensor.
    Monochrome = 0,

    /// Multiple-plane color sensor.
    Color = 1,

    /// Single-plane Bayer matrix RGGB sensor.
    RGGB = 2,

    /// Single-plane Bayer matrix CMYG sensor.
    CMYG = 3,

    /// Single-plane Bayer matrix CMYG2 sensor.
    CMYG2 = 4,

    /// Single-plane Bayer matrix LRGB sensor.
    LRGB = 5,
}
