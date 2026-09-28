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
        let mut conformu = cmd!(r"C:\Program Files\ASCOM\ConformU", "conformu")
            .arg(self.as_arg())
            .args(
                settings_file
                    .into_iter()
                    .flat_map(|path| [std::ffi::OsStr::new("--settingsfile"), path.as_os_str()]),
            )
            .arg(device_url.as_str())
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
/// # Example
///
/// ```ignore
/// // Run with ConformU's own per-user settings
/// ConformUTestBuilder::new::<dyn Switch>(server_url, 0)?.run().await?;
///
/// // Run with a custom settings file, e.g. shorter Switch delays for faster CI
/// ConformUTestBuilder::new::<dyn Switch>(server_url, 0)?
///     .settings_file("test-fixtures/conformu-settings.json")
///     .run()
///     .await?;
/// ```
#[derive(Debug)]
pub struct ConformUTestBuilder {
    device_url: Url,
    settings_file: Option<PathBuf>,
}

impl ConformUTestBuilder {
    /// Create a new builder for testing a device at the specified URL.
    ///
    /// This assumes that ConformU is installed and available on PATH or in the
    /// default installation location.
    #[allow(private_bounds)]
    pub fn new<T: ?Sized + RetrieavableDevice>(
        server_url: impl IntoUrl + Debug,
        device_number: usize,
    ) -> eyre::Result<Self> {
        let url = server_url
            .into_url()?
            .join(&format!("api/v1/{ty}/{device_number}", ty = T::TYPE))?;

        Ok(Self {
            device_url: url,
            settings_file: None,
        })
    }

    /// Set a custom ConformU settings file, passed to both runs as `--settingsfile`.
    ///
    /// The file must exist and hold the integer `"SettingsCompatibilityVersion": 1`
    /// at its root. ConformU finds it by looking for the literal text
    /// `"SettingsCompatibilityVersion":` (exact case, nothing between the closing
    /// quote and the colon); without that text it silently renames the file to
    /// `<file>.prereleaseversion` and writes a file of defaults in its place, and
    /// any other integer version is moved aside as `<file>.unknownversion` the
    /// same way. A file it cannot parse (a comment, a trailing comma, a wrongly
    /// typed value) is ignored for the run, with only an `ERROR` line in its
    /// output.
    ///
    /// List only the settings you change: every property the file omits keeps
    /// ConformU's default, though a dictionary such as `DomeTests` is replaced
    /// whole. For example, to shorten the pauses ConformU makes after each
    /// `GetSwitch` / `GetSwitchValue` read (default 500 ms) and each `SetSwitch` /
    /// `SetSwitchValue` write (default 3000 ms):
    ///
    /// ```json
    /// { "SettingsCompatibilityVersion": 1, "SwitchReadDelay": 50, "SwitchWriteDelay": 100 }
    /// ```
    ///
    /// [`run`](Self::run) uses `alpacaprotocol <url>` and `conformance <url>`, which
    /// call ConformU's `SetFullTest()` after loading the file. That resets every
    /// `[MandatoryInFullTest]` setting and the `TelescopeTests` selection to their
    /// full-test values, so the file cannot turn off switch writes, the opening
    /// of a dome's shutter, or the side-of-pier tests (whether they run still
    /// depends on the device). Delays, timeouts and tolerances are honoured, and
    /// a loosened tolerance raises no alert. Of what the file can still change,
    /// only a `false` entry in `DomeTests` or turning off
    /// `AlpacaConfiguration.StrictCasing` raises a configuration alert, which
    /// makes `run` fail.
    ///
    /// Without a settings file, ConformU uses its per-user settings
    /// (`<LocalApplicationData>/conform/conform.settings`, shared with its GUI), so
    /// the run is not hermetic: on 64-bit Windows, for example, a saved *Run as
    /// 32-bit* setting makes ConformU start a detached 32-bit copy of itself and
    /// exit 0 at once, so `run` passes without seeing any result while that copy
    /// tests the device unobserved. Pass a file holding just
    /// `{"SettingsCompatibilityVersion": 1}` to start from ConformU's defaults.
    #[must_use]
    pub fn settings_file(mut self, path: impl AsRef<Path>) -> Self {
        self.settings_file = Some(path.as_ref().to_path_buf());
        self
    }

    /// Run all ConformU tests (`AlpacaProtocol` and `Conformance`).
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

/// Run all the ConformU tests against the device at the specified URL.
///
/// This assumes that ConformU is installed and available on PATH or in the
/// default installation location.
///
/// For more configuration options, use [`ConformUTestBuilder`].
#[tracing::instrument(level = "error", fields(ty = ?T::TYPE))]
#[allow(private_bounds)]
pub async fn run_conformu_tests<T: ?Sized + RetrieavableDevice>(
    server_url: impl IntoUrl + Debug,
    device_number: usize,
) -> eyre::Result<()> {
    ConformUTestBuilder::new::<T>(server_url, device_number)?
        .run()
        .await
}
