use crate::{ASCOMError, ASCOMResult};
use macro_rules_attribute::apply;
use std::fmt::Debug;

/// ASCOM Methods Common To All Devices.
#[apply(rpc_trait)]
pub trait Device: Debug + Send + Sync {
    /// Actions and SupportedActions are a standardised means for drivers to extend functionality beyond the built-in capabilities of the ASCOM device interfaces.
    ///
    /// The key advantage of using Actions is that drivers can expose any device specific functionality required. The downside is that, in order to use these unique features, every application author would need to create bespoke code to present or exploit them.
    ///
    /// The Action parameter and return strings are deceptively simple, but can support transmission of arbitrarily complex data structures, for example through JSON encoding.
    ///
    /// This capability will be of primary value to:
    ///  * bespoke software and hardware configurations where a single entity controls both the consuming application software and the hardware / driver environment
    ///  * a group of application and device authors to quickly formulate and try out new interface capabilities without requiring an immediate change to the ASCOM device interface, which will take a lot longer than just agreeing a name, input parameters and a standard response for an Action command
    ///
    ///
    /// The list of Action commands supported by a driver can be discovered through the SupportedActions property.
    ///
    /// This method should return an error message and NotImplementedException error number (0x400) if the driver just implements the standard ASCOM device methods and has no bespoke, unique, functionality.
    #[http("action", method = Put)]
    async fn action(
        &self,

        #[http("Action")] action: String,

        #[http("Parameters")] parameters: String,
    ) -> ASCOMResult<String> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Transmits an arbitrary string to the device and does not wait for a response.
    ///
    /// Optionally, protocol framing characters may be added to the string before transmission.
    #[http("commandblind", method = Put)]
    #[deprecated(note = "Use the more flexible Action and SupportedActions mechanic.")]
    async fn command_blind(
        &self,

        #[http("Command")] command: String,

        #[http("Raw")] raw: String,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Transmits an arbitrary string to the device and waits for a boolean response.
    ///
    /// Optionally, protocol framing characters may be added to the string before transmission.
    #[http("commandbool", method = Put)]
    #[deprecated(note = "Use the more flexible Action and SupportedActions mechanic.")]
    async fn command_bool(
        &self,

        #[http("Command")] command: String,

        #[http("Raw")] raw: String,
    ) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Transmits an arbitrary string to the device and waits for a string response.
    ///
    /// Optionally, protocol framing characters may be added to the string before transmission.
    #[http("commandstring", method = Put)]
    #[deprecated(note = "Use the more flexible Action and SupportedActions mechanic.")]
    async fn command_string(
        &self,

        #[http("Command")] command: String,

        #[http("Raw")] raw: String,
    ) -> ASCOMResult<String> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Retrieves the connected state of the device.
    #[http("connected", method = Get)]
    async fn connected(&self) -> ASCOMResult<bool>;

    /// Sets the connected state of the device.
    #[http("connected", method = Put)]
    async fn set_connected(&self, #[http("Connected")] connected: bool) -> ASCOMResult<()>;

    /// Returns true while the device is connecting or disconnecting.
    ///
    /// _Platform 7 onward._
    #[http("connecting", method = Get)]
    async fn connecting(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The description of the device.
    #[http("description", method = Get)]
    async fn description(&self) -> ASCOMResult<String>;

    /// The description of the driver.
    #[http("driverinfo", method = Get)]
    async fn driver_info(&self) -> ASCOMResult<String>;

    /// A string containing only the major and minor version of the driver.
    ///
    /// This must be in the form "n.n".
    #[http("driverversion", method = Get)]
    async fn driver_version(&self) -> ASCOMResult<String>;

    /// The name of the device.
    #[http("name", method = Get)]
    async fn name(&self) -> ASCOMResult<String> {
        Ok(self.static_name().to_owned())
    }

    /// Returns the list of action names supported by this driver.
    #[http("supportedactions", method = Get)]
    async fn supported_actions(&self) -> ASCOMResult<Vec<String>> {
        Ok(vec![])
    }
}
