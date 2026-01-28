use crate::api::RetrieavableDevice;
use reqwest::{IntoUrl, Url};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

/// The kind of test to run with ConformU.
#[derive(Debug, Clone, Copy)]
enum ConformU {
    /// Check the specified Alpaca device for Alpaca protocol conformance.
    AlpacaProtocol,

    /// Check the specified device for ASCOM device interface conformance with all tests enabled.
    Conformance,
}

impl ConformU {
    const fn as_arg(self) -> &'static str {
        match self {
            Self::AlpacaProtocol => "alpacaprotocol",
            Self::Conformance => "conformance",
        }
    }

    /// Run the specified test with ConformU against the specified device URL.
    #[tracing::instrument(level = "error", skip(device_url, settings_file))]
    async fn test(self, device_url: &Url, settings_file: Option<&Path>) -> eyre::Result<()> {
        use std::ffi::OsString;

        // Build args list - must be done before cmd! macro to avoid borrow issues
        let mut args: Vec<OsString> = vec![self.as_arg().into()];
        if let Some(path) = settings_file {
            args.push("--settingsfile".into());
            args.push(path.into());
        }
        args.push(device_url.as_str().into());

        let mut conformu = cmd!(r"C:\Program Files\ASCOM\ConformU", "conformu")
            .args(&args)
            .stdout(Stdio::piped())
            .spawn()?;

        let output = conformu.stdout.take().expect("stdout should be piped");

        let mut lines = BufReader::new(output).lines();

        while let Some(line) = lines.next_line().await? {
            if !self.parse_log_line(&line) {
                tracing::debug!("{line}");
            }
        }

        let exit_status = conformu.wait().await?;

        eyre::ensure!(
            exit_status.success(),
            "ConformU exited with an error code: {exit_status}"
        );

        Ok(())
    }

    /// This function parses log lines from ConformU.
    ///
    /// This is fragile, but ConformU doesn't provide structured output. Instead, we use known widths of the fields to parse them.
    ///
    /// See <https://github.com/ASCOMInitiative/ConformU/blob/cb32ac3d230e99636c639ccf4ac68dd3ae955c26/ConformU/AlpacaProtocolTestManager.cs>.
    #[expect(clippy::cognitive_complexity)]
    fn parse_log_line(self, mut line: &str) -> bool {
        // Skip .NET stacktraces.
        if line.starts_with("   at ") {
            return true;
        }

        // skip date and time before doing any other checks
        line = line.get(13..).unwrap_or(line).trim_ascii_end();

        // Skip empty lines.
        if line.is_empty() {
            return true;
        }

        let Some(mut method) =
            split_with_whitespace(&mut line, 35).filter(|&method| !method.is_empty())
        else {
            return false;
        };

        let Some(outcome) = split_with_whitespace(&mut line, 8) else {
            return false;
        };

        // `tracing` crate doesn't support variable-based log levels, so we have to manually map them.
        macro_rules! trace_outcome {
            (@impl $args:tt) => {
                match outcome {
                    "OK" => tracing::trace! $args,
                    "INFO" => tracing::info! $args,
                    "WARN" => tracing::warn! $args,
                    "DEBUG" | "" => tracing::debug! $args,
                    "ISSUE" | "ERROR" => tracing::error! $args,
                    _ => return false,
                }
            };

            ($target:literal, $($args:tt)*) => {
                trace_outcome!(@impl (target: concat!("ascom_alpaca::conformu::", $target), $($args)*, "{line}"))
            };
        }

        match self {
            Self::AlpacaProtocol => {
                // Example log line (after date):
                // GET Azimuth                         OK       Different ClientID casing - The expected ClientTransactionID was returned: 67890

                let Some(http_method) = split_with_whitespace(&mut method, 3)
                    .filter(|&http_method| matches!(http_method, "GET" | "PUT" | ""))
                else {
                    return false;
                };

                let test;

                (test, line) = match line.split_once(" - ") {
                    Some((test, line)) => (Some(test), line),
                    None => (None, line),
                };

                trace_outcome!(
                    "alpaca",
                    test,
                    method,
                    outcome = (!outcome.is_empty()).then_some(outcome),
                    http_method
                );
            }

            Self::Conformance => {
                // Example log line (after date):
                // DeclinationRate Write               INFO     Configured offset test duration: 10 seconds.

                trace_outcome!(
                    "conformance",
                    method,
                    outcome = (!outcome.is_empty()).then_some(outcome)
                );
            }
        }

        true
    }
}

fn split_with_whitespace<'line>(line: &mut &'line str, len: usize) -> Option<&'line str> {
    if *line.as_bytes().get(len)? != b' ' {
        return None;
    }
    let part = line[..len].trim_end_matches(' ');
    *line = &line[len + 1..];
    Some(part)
}

/// Builder for configuring and running ConformU tests.
///
/// Created via [`conformu_tests`].
#[derive(Debug)]
pub struct ConformUTestBuilder {
    device_url: Url,
    settings_file: Option<PathBuf>,
}

impl ConformUTestBuilder {
    /// Create a new builder for testing a device at the specified URL.
    fn new(device_url: Url) -> Self {
        Self {
            device_url,
            settings_file: None,
        }
    }

    /// Set a custom ConformU settings file.
    ///
    /// # Important: Complete Settings File Required
    ///
    /// ConformU requires a **complete** settings file with all properties.
    /// Partial files containing only a few settings will be silently ignored
    /// and overwritten with defaults.
    ///
    /// ## Generating a Default Settings Template
    ///
    /// To generate a complete settings file with all default values, run ConformU
    /// with an empty JSON object file. ConformU will populate it with all defaults:
    ///
    /// ```bash
    /// echo "{}" > conformu-settings.json
    /// conformu conformance --settingsfile conformu-settings.json http://localhost:99999/api/v1/switch/0
    /// # The command will fail (no server), but conformu-settings.json now has all defaults
    /// ```
    ///
    /// You can then edit the generated file to customize specific values
    /// (e.g., reduce `SwitchReadDelay` and `SwitchWriteDelay` for faster CI).
    ///
    /// ## Switch Testing Performance
    ///
    /// For Switch device tests, the default delays are:
    /// - `SwitchReadDelay`: 500ms
    /// - `SwitchWriteDelay`: 3000ms
    ///
    /// For CI environments, reducing these (e.g., to 50ms and 100ms) can cut test
    /// time from ~8 minutes to ~35 seconds.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Use a pre-configured settings file with reduced delays
    /// conformu_tests::<dyn Switch>(server_url, 0)?
    ///     .settings_file("test-fixtures/conformu-settings.json")
    ///     .run()
    ///     .await?;
    /// ```
    pub fn settings_file(mut self, path: impl AsRef<Path>) -> Self {
        self.settings_file = Some(path.as_ref().to_path_buf());
        self
    }

    /// Run all ConformU tests (AlpacaProtocol and Conformance).
    #[tracing::instrument(level = "error", skip(self))]
    pub async fn run(self) -> eyre::Result<()> {
        // Must be executed serially as they operate on the same device.
        ConformU::AlpacaProtocol
            .test(&self.device_url, self.settings_file.as_deref())
            .await?;
        ConformU::Conformance
            .test(&self.device_url, self.settings_file.as_deref())
            .await
    }
}

/// Create a builder for running ConformU tests against the device at the specified URL.
///
/// This assumes that ConformU is installed and available on PATH or in the
/// default installation location.
///
/// # Example
///
/// ```ignore
/// // Run with default settings
/// conformu_tests::<dyn Switch>(server_url, 0)?.run().await?;
///
/// // Run with custom settings file for faster CI
/// // Note: ConformU requires a COMPLETE settings file - see settings_file() docs
/// conformu_tests::<dyn Switch>(server_url, 0)?
///     .settings_file("test-fixtures/conformu-settings.json")
///     .run()
///     .await?;
/// ```
#[allow(private_bounds)]
pub fn conformu_tests<T: ?Sized + RetrieavableDevice>(
    server_url: impl IntoUrl + Debug,
    device_number: usize,
) -> eyre::Result<ConformUTestBuilder> {
    let url = server_url
        .into_url()?
        .join(&format!("api/v1/{ty}/{device_number}", ty = T::TYPE))?;

    Ok(ConformUTestBuilder::new(url))
}

/// Run all the ConformU tests against the device at the specified URL.
///
/// This assumes that ConformU is installed and available on PATH or in the
/// default installation location.
///
/// For more configuration options, use [`conformu_tests`] to get a builder.
#[tracing::instrument(level = "error", fields(ty = ?T::TYPE))]
#[allow(private_bounds)]
pub async fn run_conformu_tests<T: ?Sized + RetrieavableDevice>(
    server_url: impl IntoUrl + Debug,
    device_number: usize,
) -> eyre::Result<()> {
    conformu_tests::<T>(server_url, device_number)?.run().await
}
