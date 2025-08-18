mod image_array;
pub use image_array::*;

pub use super::camera_telescope_shared::GuideDirection;

use super::Device;
use macro_rules_attribute::apply;
use crate::{ASCOMError, ASCOMResult};
use serde_repr::{Deserialize_repr, Serialize_repr};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use super::camera_telescope_shared::{TimeRepr, Fits};

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
    async fn set_bin_x(&self, #[http("BinX")] bin_x: i32) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the binning factor for the Y axis.
    #[http("biny", method = Get)]
    async fn bin_y(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the binning factor for the Y axis.
    #[http("biny", method = Put)]
    async fn set_bin_y(&self, #[http("BinY")] bin_y: i32) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current camera operational state as an integer.
    #[http("camerastate", method = Get)]
    async fn camera_state(&self) -> ASCOMResult<CameraState> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the width of the CCD camera chip in unbinned pixels.
    #[http("cameraxsize", method = Get)]
    async fn camera_xsize(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the height of the CCD camera chip in unbinned pixels.
    #[http("cameraysize", method = Get)]
    async fn camera_ysize(&self) -> ASCOMResult<i32> {
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
    #[http("ccdtemperature", method = Get)]
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
    async fn set_cooler_on(&self, #[http("CoolerOn")] cooler_on: bool) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the present cooler power level, in percent.
    #[http("coolerpower", method = Get)]
    async fn cooler_power(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the gain of the camera in photoelectrons per A/D unit.
    #[http("electronsperadu", method = Get)]
    async fn electrons_per_adu(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the maximum exposure time supported by StartExposure.
    #[http("exposuremax", method = Get)]
    async fn exposure_max(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the Minimium exposure time in seconds that the camera supports through StartExposure.
    #[http("exposuremin", method = Get)]
    async fn exposure_min(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the smallest increment in exposure time supported by StartExposure.
    #[http("exposureresolution", method = Get)]
    async fn exposure_resolution(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns whenther Fast Readout Mode is enabled.
    #[http("fastreadout", method = Get)]
    async fn fast_readout(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets whether Fast Readout Mode is enabled.
    #[http("fastreadout", method = Put)]
    async fn set_fast_readout(&self, #[http("FastReadout")] fast_readout: bool) -> ASCOMResult {
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
    async fn set_gain(&self, #[http("Gain")] gain: i32) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the maximum value of Gain.
    #[http("gainmax", method = Get)]
    async fn gain_max(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the Minimum value of Gain.
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
    async fn has_shutter(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current heat sink temperature (called "ambient temperature" by some manufacturers) in degrees Celsius.
    #[http("heatsinktemperature", method = Get)]
    async fn heat_sink_temperature(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns an array of 32bit integers containing the pixel values from the last exposure.
    ///
    /// This call can return either a 2 dimension (monochrome images) or 3 dimension (colour or multi-plane images) array  of size `NumX * NumY` or `NumX * NumY * NumPlanes`. Where applicable, the size of `NumPlanes` has to be determined by inspection of the returned array.
    ///
    /// Since 32bit integers are always returned by this call, the returned JSON `Type` value is always 2 (integer).
    ///
    /// When de-serialising to an object it is essential to know the array `Rank` beforehand so that the correct data class can be used. This can be achieved through a regular expression or by direct parsing  of the returned JSON string to extract the `Type` and `Rank` values before de-serialising.
    ///
    /// This regular expression accomplishes the extraction into two named groups `Type` and `Rank`, which can then be used to select the correct de-serialisation data class:
    ///
    /// `^*"Type":(?<Type>\d*),"Rank":(?<Rank>\d*)`
    ///
    /// When the `SensorType` is Monochrome, RGGB, CMYG, CMYG2 or LRGB, the serialised JSON array should have 2 dimensions. For example, the returned array should appear as below if `NumX = 7`, `NumY = 5`  and `Pxy` represents the pixel value at the zero based position `x` across and `y` down the image with the origin in the top left corner of the image.
    ///
    /// Please note that this is "column-major" order (column changes most rapidly) from the image's row and column perspective, while, from the array's perspective, serialisation is actually effected in "row-major" order (rightmost index changes most rapidly).  This unintuitive outcome arises because the ASCOM Camera Interface specification defines the image column dimension as the rightmost array dimension.
    ///
    /// ```text
    /// [
    ///   [P00,P01,P02,P03,P04],
    ///
    ///   [P10,P11,P12,P13,P14],
    ///
    ///   [P20,P21,P22,P23,P24],
    ///
    ///   [P30,P31,P32,P33,P34],
    ///
    ///   [P40,P41,P42,P43,P44],
    ///
    ///   [P50,P51,P52,P53,P54],
    ///
    ///   [P60,P61,P62,P63,P64],
    ///
    ///   …
    /// ]
    /// ```
    ///
    /// When the `SensorType` is Color, the serialised JSON array will have 3 dimensions. For example, the returned array should appear as below if `NumX = 7`, `NumY = 5`  and `Rxy`, `Gxy` and `Bxy` represent the red, green and blue pixel values at the zero based position x across and y down the image with the origin in the top left corner of the image.  Please see note above regarding element ordering.
    ///
    /// ```text
    /// [
    ///   [[R00,G00,B00],[R01,G01,B01],[R02,G02,B02],[R03,G03,B03],[R04,G04,B04]],
    ///
    ///
    ///   [[R10,G10,B10],[R11,G11,B11],[R12,G12,B12],[R13,G13,B13],[R14,G14,B14]],
    ///
    ///
    ///   [[R20,G20,B20],[R21,G21,B21],[R22,G22,B22],[R23,G23,B23],[R24,G24,B24]],
    ///
    ///
    ///   [[R30,G30,B30],[R31,G31,B31],[R32,G32,B32],[R33,G33,B33],[R34,G34,B34]],
    ///
    ///
    ///   [[R40,G40,B40],[R41,G41,B41],[R42,G42,B42],[R43,G43,B43],[R44,G44,B44]],
    ///
    ///
    ///   [[R50,G50,B50],[R51,G51,B51],[R52,G52,B52],[R53,G53,B53],[R54,G54,B54]],
    ///
    ///
    ///   [[R60,G60,B60],[R61,G61,B61],[R62,G62,B62],[R63,G63,B63],[R64,G64,B64]],
    ///
    ///   …
    /// ]
    /// ```
    /// ## Performance
    /// Returning an image from an Alpaca device as a JSON array is very inefficient and can result in delays of 30 or more seconds while client and device process and send the huge JSON string over the network.  A new, much faster mechanic called ImageBytes - [Alpaca ImageBytes Concepts and Implementation](https://www.ascom-standards.org/Developer/AlpacaImageBytes.pdf) has been developed that sends data as a binary byte stream and can offer a 10 to 20 fold reduction in transfer time.  It is strongly recommended that Alpaca Cameras implement the ImageBytes mechanic as well as the JSON mechanic.
    #[http("imagearray", method = Get)]
    async fn image_array(&self) -> ASCOMResult<ImageArray> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns a flag indicating whether the image is ready to be downloaded from the camera.
    #[http("imageready", method = Get)]
    async fn image_ready(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns a flag indicating whether the camera is currrently in a PulseGuide operation.
    #[http("ispulseguiding", method = Get)]
    async fn is_pulse_guiding(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Reports the actual exposure duration in seconds (i.e. shutter open time).
    #[http("lastexposureduration", method = Get)]
    async fn last_exposure_duration(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Reports the actual exposure start in the FITS-standard CCYY-MM-DDThh:mm:ss[.sss...] format.
    ///
    /// The time must be UTC.
    #[http("lastexposurestarttime", method = Get, via = TimeRepr<Fits>)]
    async fn last_exposure_start_time(&self) -> ASCOMResult<std::time::SystemTime> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Reports the maximum ADU value the camera can produce.
    #[http("maxadu", method = Get)]
    async fn max_adu(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

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

    /// Returns the current subframe width, if binning is active, value is in binned pixels.
    #[http("numx", method = Get)]
    async fn num_x(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the current subframe width.
    #[http("numx", method = Put)]
    async fn set_num_x(&self, #[http("NumX")] num_x: i32) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current subframe height, if binning is active, value is in binned pixels.
    #[http("numy", method = Get)]
    async fn num_y(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the current subframe height.
    #[http("numy", method = Put)]
    async fn set_num_y(&self, #[http("NumY")] num_y: i32) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the camera's offset (OFFSET VALUE MODE) OR the index of the selected camera offset description in the offsets array (OFFSETS INDEX MODE).
    #[http("offset", method = Get)]
    async fn offset(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the camera's offset (OFFSET VALUE MODE) OR the index of the selected camera offset description in the offsets array (OFFSETS INDEX MODE).
    #[http("offset", method = Put)]
    async fn set_offset(&self, #[http("Offset")] offset: i32) -> ASCOMResult {
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
    #[http("percentcompleted", method = Get)]
    async fn percent_completed(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the width of the CCD chip pixels in microns.
    #[http("pixelsizex", method = Get)]
    async fn pixel_size_x(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the Height of the CCD chip pixels in microns.
    #[http("pixelsizey", method = Get)]
    async fn pixel_size_y(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// ReadoutMode is an index into the array ReadoutModes and returns the desired readout mode for the camera.
    ///
    /// Defaults to 0 if not set.
    #[http("readoutmode", method = Get)]
    async fn readout_mode(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the ReadoutMode as an index into the array ReadoutModes.
    #[http("readoutmode", method = Put)]
    async fn set_readout_mode(&self, #[http("ReadoutMode")] readout_mode: i32) -> ASCOMResult {
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
    ) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current subframe start position for the X axis (0 based).
    ///
    /// If binning is active, value is in binned pixels.
    #[http("startx", method = Get)]
    async fn start_x(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the current subframe X axis start position in binned pixels.
    #[http("startx", method = Put)]
    async fn set_start_x(&self, #[http("StartX")] start_x: i32) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current subframe start position for the Y axis (0 based).
    ///
    /// If binning is active, value is in binned pixels.
    #[http("starty", method = Get)]
    async fn start_y(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the current subframe Y axis start position in binned pixels.
    #[http("starty", method = Put)]
    async fn set_start_y(&self, #[http("StartY")] start_y: i32) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

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
    ) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Aborts the current exposure, if any, and returns the camera to Idle state.
    #[http("abortexposure", method = Put)]
    async fn abort_exposure(&self) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Activates the Camera's mount control sytem to instruct the mount to move in a particular direction for a given period of time.
    #[http("pulseguide", method = Put)]
    async fn pulse_guide(
        &self,

        #[http("Direction")] direction: GuideDirection,

        #[http("Duration")] duration: i32,
    ) -> ASCOMResult {
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
    ) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Stops the current exposure, if any.
    ///
    /// If an exposure is in progress, the readout process is initiated. Ignored if readout is already in process.
    #[http("stopexposure", method = Put)]
    async fn stop_exposure(&self) -> ASCOMResult {
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
