use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

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
        let mut conformu = Command::new(r"C:\Program Files\ASCOM\ConformU\conformu.exe")
            .arg(self.as_arg())
            .arg(device_url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()?;

        let output = conformu.stdout.take().expect("stdout should be piped");

        let reader = BufReader::new(output);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            // This is fragile, but ConformU doesn't provide structured output.
            // Use known widths of the fields to parse them.
            // https://github.com/ASCOMInitiative/ConformU/blob/cb32ac3d230e99636c639ccf4ac68dd3ae955c26/ConformU/AlpacaProtocolTestManager.cs

            // Skip .NET stacktraces.
            if line.starts_with("   at ") {
                continue;
            }

            let line = match self {
                // skip date and time before doing any other checks
                Self::Conformance => line.get(13..).unwrap_or(&line),
                // In protocol tests, the date and time are not present
                Self::AlpacaProtocol => &line,
            }
            .trim_ascii_end();

            // Skip empty lines.
            if line.is_empty() {
                continue;
            }

            if self.parse_log_line(line).is_none() {
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

    #[allow(clippy::cognitive_complexity)]
    fn parse_log_line(self, mut line: &str) -> Option<()> {
        let outcome;

        macro_rules! trace_outcome {
				(@impl $args:tt) => {
					match outcome {
						"OK" => tracing::trace! $args,
						"INFO" => tracing::info! $args,
						"WARN" => tracing::warn! $args,
						"DEBUG" | "" => tracing::debug! $args,
						"ISSUE" | "ERROR" => tracing::error! $args,
						_ => return None,
					}
				};

				($target:literal, $($args:tt)*) => {
					trace_outcome!(@impl (target: concat!("ascom_alpaca::conformu::", $target), $($args)*, "{line}"))
				};
			}

        match self {
            Self::AlpacaProtocol => {
                let http_method = split_with_whitespace(&mut line, 3)
                    .filter(|&http_method| matches!(http_method, "GET" | "PUT" | ""))?;

                let method = split_with_whitespace(&mut line, 25)?;

                outcome = split_with_whitespace(&mut line, 6)?;

                let test;

                (test, line) = match line.split_once(" - ") {
                    Some((test, line)) => (Some(test), line),
                    None => (None, line),
                };

                trace_outcome!("alpaca", test, method, outcome, http_method);
            }

            Self::Conformance => {
                let method =
                    split_with_whitespace(&mut line, 35).filter(|&method| !method.is_empty())?;

                outcome = split_with_whitespace(&mut line, 8)?;

                trace_outcome!("conformance", method, outcome);
            }
        }

        Some(())
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
