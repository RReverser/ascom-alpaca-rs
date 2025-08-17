use crate::api::{DeviceType, ServerInfo};
use crate::server::CargoServerInfo;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

// A stable alternative to `std::fmt::from_fn`.
// TODO: use the std version when it's stabilized.
struct FmtFromFn<F: Fn(&mut Formatter<'_>) -> fmt::Result>(F);

impl<F: Fn(&mut Formatter<'_>) -> fmt::Result> Display for FmtFromFn<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (self.0)(f)
    }
}

// Yeah, yeah, I didn't quite like any of existing templating libraries - mostly because they're doing way more than what I need - so wrote my own macro.
//
// If there's a similarly tiny one out there with no bells and whistles, I'm happy to switch.
macro_rules! html {
  ($tag:ident $({ $($attr:ident: $value:expr),* $(,)? })? $(, $inner:expr)*) => {
    FmtFromFn(move |f| {
        let tag = stringify!($tag);
        write!(f, "<{tag}")?;
        $($(
          write!(f, " {}={:?}", stringify!($attr), $value)?;
        )*)?
        writeln!(f, ">")?;
        $(
          write!(f, "{}", $inner)?;
        )*
        writeln!(f, "</{tag}>")
    })
  };
}

fn iter_fmt<T, O: Display>(
    iter: impl Copy + IntoIterator<Item = T>,
    to_display: impl Fn(T) -> O,
) -> impl Display {
    FmtFromFn(move |f| {
        for item in iter {
            write!(f, "{}", to_display(item))?;
        }
        Ok(())
    })
}

const CSS: &str = "
body {
  font-family: Arial, sans-serif;
  margin: 0;
  padding: 0;
  text-align: center;
}
h1,
figure {
  margin: 1em auto;
  width: 90vw;
}
h1 {
  color: #333;
  font-size: 2.5em;
}
figure {
  background-color: #fff;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3), 0 -2px 2px rgba(0, 0, 0, 0.3);
}
figcaption,
li>a {
  margin: 0;
  padding: 0.625em;
}
figcaption {
  background-color: #f5f5f5;
  color: #333;
  font-size: 1.2em;
}
ul {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
}
li:first-child {
  border-top: 1px solid #ccc;
}
li {
  border-bottom: 1px solid #ccc;
}
li>a {
  color: #333;
  text-decoration: none;
  display: block;
  transition: background-color 0.3s ease;
}
li>a:hover {
  background-color: #f5f5f5;
}
";

pub(super) struct SetupPage<'ctx> {
    pub server_info: &'ctx ServerInfo,
    pub grouped_devices: BTreeMap<DeviceType, Vec<(usize, String)>>,
}

fn group_to_html(group_ty: DeviceType, group: &[(usize, String)]) -> impl '_ + Display {
    html!(
        figure,
        html!(
            figcaption,
            match group_ty {
                #[cfg(feature = "camera")]
                DeviceType::Camera => "Cameras",
                #[cfg(feature = "dome")]
                DeviceType::Dome => "Domes",
                #[cfg(feature = "filterwheel")]
                DeviceType::FilterWheel => "Filter wheels",
                #[cfg(feature = "focuser")]
                DeviceType::Focuser => "Focusers",
                #[cfg(feature = "observingconditions")]
                DeviceType::ObservingConditions => "Weather stations",
                #[cfg(feature = "rotator")]
                DeviceType::Rotator => "Rotators",
                #[cfg(feature = "safetymonitor")]
                DeviceType::SafetyMonitor => "Safety monitors",
                #[cfg(feature = "switch")]
                DeviceType::Switch => "Switches",
                #[cfg(feature = "telescope")]
                DeviceType::Telescope => "Telescopes",
                #[cfg(feature = "covercalibrator")]
                DeviceType::CoverCalibrator => "Cover calibrators",
            }
        ),
        html!(
            ul,
            iter_fmt(group, |(number, name)| html!(
                li,
                html!(
                    a {
                        href: format_args!("/api/v1/{group_ty}/{number}/setup")
                    },
                    name
                )
            ))
        )
    )
}

impl Display for SetupPage<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "<!DOCTYPE html>")?;

        html!(
            html { lang: "en" },
            html!(
                head,
                html!(meta { charset: "UTF-8" }),
                html!(title, &self.server_info.server_name),
                html!(style, CSS)
            ),
            html!(
                body,
                html!(h2, "Registered devices"),
                FmtFromFn(|f| {
                    if self.grouped_devices.is_empty() {
                        return html!(p, "No devices are registered on this server.").fmt(f);
                    }

                    iter_fmt(&self.grouped_devices, |(&group_ty, group)| {
                        group_to_html(group_ty, group)
                    })
                    .fmt(f)
                })
            ),
            html!(
                footer,
                html!(
                    p,
                    "This is an ",
                    html!(
                        a {
                            href: "https://ascom-standards.org/AlpacaDeveloper/Index.htm"
                        },
                        "ASCOM Alpaca"
                    ),
                    " server ",
                    &self.server_info
                ),
                html!(p, "Built with ", CargoServerInfo!())
            )
        )
        .fmt(f)
    }
}
