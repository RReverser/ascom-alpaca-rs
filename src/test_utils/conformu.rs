use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

/// The kind of test to run with ConformU.
#[derive(Debug, Clone, Copy)]
pub enum ConformU {
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
    pub async fn run(self, device_url: &str) -> eyre::Result<()> {
        let mut conformu = cmd!(r"C:\Program Files\ASCOM\ConformU", "conformu")
            .arg(self.as_arg())
            .arg(device_url)
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
    #[allow(clippy::cognitive_complexity)]
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
