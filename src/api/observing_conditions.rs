use super::Device;
use crate::{ASCOMError, ASCOMResult};
use macro_rules_attribute::apply;

/// ObservingConditions Specific Methods.
#[apply(rpc_trait)]
pub trait ObservingConditions: Device + Send + Sync {
    /// Gets the time period over which observations will be averaged.
    #[http("averageperiod", method = Get)]
    async fn average_period(&self) -> ASCOMResult<f64>;

    /// Sets the time period over which observations will be averaged.
    #[http("averageperiod", method = Put)]
    async fn set_average_period(
        &self,

        #[http("AveragePeriod")] average_period: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the percentage of the sky obscured by cloud.
    #[http("cloudcover", method = Get, device_state = "CloudCover")]
    async fn cloud_cover(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the atmospheric dew point at the observatory reported in °C.
    #[http("dewpoint", method = Get, device_state = "DewPoint")]
    async fn dew_point(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the atmospheric  humidity (%) at the observatory.
    #[http("humidity", method = Get, device_state = "Humidity")]
    async fn humidity(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the atmospheric pressure in hectoPascals at the observatory's altitude - NOT reduced to sea level.
    #[http("pressure", method = Get, device_state = "Pressure")]
    async fn pressure(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the rain rate (mm/hour) at the observatory.
    #[http("rainrate", method = Get, device_state = "RainRate")]
    async fn rain_rate(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the sky brightness at the observatory (Lux).
    #[http("skybrightness", method = Get, device_state = "SkyBrightness")]
    async fn sky_brightness(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the sky quality at the observatory (magnitudes per square arc second).
    #[http("skyquality", method = Get, device_state = "SkyQuality")]
    async fn sky_quality(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the sky temperature(°C) at the observatory.
    #[http("skytemperature", method = Get, device_state = "SkyTemperature")]
    async fn sky_temperature(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the seeing at the observatory measured as star full width half maximum (FWHM) in arc secs.
    #[http("starfwhm", method = Get)]
    async fn star_fwhm(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the temperature(°C) at the observatory.
    #[http("temperature", method = Get, device_state = "Temperature")]
    async fn temperature(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the wind direction.
    ///
    /// The returned value must be between 0.0 and 360.0, interpreted according to the metereological standard, where a special value of 0.0 is returned when the wind speed is 0.0. Wind direction is measured clockwise from north, through east, where East=90.0, South=180.0, West=270.0 and North=360.0.
    #[http("winddirection", method = Get, device_state = "WindDirection")]
    async fn wind_direction(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the peak 3 second wind gust(m/s) at the observatory over the last 2 minutes.
    #[http("windgust", method = Get, device_state = "WindGust")]
    async fn wind_gust(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the wind speed(m/s) at the observatory.
    #[http("windspeed", method = Get, device_state = "WindSpeed")]
    async fn wind_speed(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Forces the driver to immediately query its attached hardware to refresh sensor values.
    #[http("refresh", method = Put)]
    async fn refresh(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets a description of the sensor with the name specified in the SensorName parameter.
    #[http("sensordescription", method = Get)]
    async fn sensor_description(
        &self,

        #[http("SensorName")] sensor_name: String,
    ) -> ASCOMResult<String> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the time since the sensor specified in the SensorName parameter was last updated.
    #[http("timesincelastupdate", method = Get)]
    async fn time_since_last_update(
        &self,

        #[http("SensorName")] sensor_name: String,
    ) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(2_i32)
    }
}
