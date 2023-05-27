/*!
This is a Rust implementation of the standard [ASCOM Alpaca API](https://ascom-standards.org/api/) for astronomy devices.

It implements main Alpaca API clients and servers, as well as transparent support for auto-discovery mechanism and `ImageBytes` encoding for camera images.

## Usage

### Compilation features

This crate defines two sets of compilation features that help to keep binary size & compilation speed in check by opting into only the features you need.

First set is along the client-server axis:

- `client`: Enables client-side access to Alpaca-capable devices.
- `server`: Allows to expose your own devices as Alpaca servers.

The second set of features is based on the device type and enables the corresponding trait:

- `all-devices`: Enables all of the below. Not recommended unless you're building a universal astronomy application.
- `camera`: Enables support for cameras via the [`Camera`](crate::api::Camera) trait.
- `covercalibrator`: Enables [...] the [`CoverCalibrator`](crate::api::CoverCalibrator) trait.
- `dome`: Enables [`Dome`](crate::api::Dome).
- `filterwheel`: Enables [`FilterWheel`](crate::api::FilterWheel).
- `focuser`: Enables [`Focuser`](crate::api::Focuser).
- `observingconditions`: Enables [`ObservingConditions`](crate::api::ObservingConditions).
- `rotator`: Enables [`Rotator`](crate::api::Rotator).
- `switch`: Enables [`Switch`](crate::api::Switch).
- `telescope`: Enables [`Telescope`](crate::api::Telescope).

Once you decided on the features you need, you can add this crate to your `Cargo.toml`. For example, if I'm implementing an Alpaca camera driver, I'd add the following to my `Cargo.toml`:

```toml
[dependencies]
ascom-alpaca = { version = "0.1", features = ["client", "camera"] }
```

### Device methods

All the device type trait methods are async and correspond to the [ASCOM Alpaca API](https://ascom-standards.org/api/). They all return [`ASCOMResult<...>`](crate::ASCOMResult).

The [`Device`](crate::api::Device) supertrait includes "ASCOM Methods Common To All Devices" from the Alpaca API, as well as a few custom metadata methods used for the device registration:

- [`fn static_name(&self) -> &str`](crate::api::Device::static_name): Returns the static device name.
- [`fn unique_id(&self) -> &str`](crate::api::Device::unique_id): Returns globally-unique device ID.

### Implementing a device server

Since async traits are not yet natively supported on stable Rust, the traits are implemented using the [async-trait](https://crates.io/crates/async-trait) crate. Other than that, you should implement trait with all the Alpaca methods as usual:

```no_run
use ascom_alpaca::ASCOMResult;
use ascom_alpaca::api::{Device, Camera};
use async_trait::async_trait;

#[derive(Debug)]
struct MyCamera {
    // ...
}

#[async_trait]
impl Device for MyCamera {
    fn static_name(&self) -> &str {
        "My Camera"
    }

    fn unique_id(&self) -> &str {
        "insert GUID here"
    }

    // ...
}

#[async_trait]
impl Camera for MyCamera {
    async fn bayer_offset_x(&self) -> ASCOMResult<i32> {
        Ok(0)
    }

    async fn bayer_offset_y(&self) -> ASCOMResult<i32> {
        Ok(0)
    }

    // ...
}
```

Any skipped methods will default to the following values:

- `can_*` feature detection methods - to `false`.
- [`Device::name`](crate::api::Device::name) - to the result of [`Device::static_name()`](crate::api::Device::static_name).
- [`Device::interface_version`](crate::api::Device::interface_version) - to `3` (latest ASCOM interface version implemented by this crate).
- [`Device::supported_actions`](crate::api::Device::supported_actions) - to an empty list.
- All other methods - to [`Err(ASCOMError::NOT_IMPLEMENTED)`](crate::ASCOMError::NOT_IMPLEMENTED). It's your responsibility to consult documentation and implement mandatory methods.

Once traits are implemented, you can create a server, register your device(s), and start listening:

```no_run
use ascom_alpaca::Server;
use ascom_alpaca::api::CargoServerInfo;
use std::convert::Infallible;

// ...implement MyCamera...
# use ascom_alpaca::{api, ASCOMResult};
# use async_trait::async_trait;
#
# #[derive(Debug)]
# struct MyCamera {}
# impl api::Device for MyCamera {
# fn static_name(&self) -> &str { todo!() }
# fn unique_id(&self) -> &str { todo!() }
# }
# impl api::Camera for MyCamera {}

#[tokio::main]
async fn main() -> eyre::Result<Infallible> {
    let mut server = Server {
        // helper macro to populate server information from your own Cargo.toml
        info: CargoServerInfo!(),
        ..Default::default()
    };

    // By default, the server will listen on dual-stack (IPv4 + IPv6) unspecified address with a randomly assigned port.
    // You can change that by modifying the `listen_addr` field:
    server.listen_addr.set_port(8000);

    // Create and register your device(s).
    server.devices.register(MyCamera { /* ... */ });

    // Start the infinite server loop.
    server.start().await
}
```

This will start both the main Alpaca server as well as an auto-discovery responder.

**Examples:**

- [`examples/camera-server.rs`](https://github.com/RReverser/ascom-alpaca-rs/blob/main/examples/camera-server.rs):
  A cross-platform example exposing your connected webcam(s) as Alpaca `Camera`s.

  Long exposures are simulated by stacking up individual frames up to the total duration.
  This approach can't provide precise requested exposure, but works well enough otherwise.
- [`star-adventurer-alpaca`](https://github.com/RReverser/star-adventurer-alpaca):
  A fork of [`jsorrell/star-adventurer-alpaca`](https://github.com/jsorrell/star-adventurer-alpaca) which implements the Alpaca API for the Star Adventurer mount over serial port.
  The original project has pretty extensive functionality and used manual implementation of the Alpaca API, so it was a good test case for porting to this library.

### Accessing devices from a client

If you know address of the device server you want to access, you can access it directly via `Client` struct:

```no_run
# #[tokio::main]
# async fn main() -> eyre::Result<()> {
use ascom_alpaca::Client;

let client = Client::new("http://localhost:8000")?;

// `get_server_info` returns high-level metadata of the server.
println!("Server info: {:#?}", client.get_server_info().await?);

// `get_devices` returns an iterator over all the devices registered on the server.
// Each is represented as a `TypedDevice` tagged enum encompassing all the device types as corresponding trait objects.
// You can either match on them to select the devices you're interested in, or, say, just print all of them:
println!("Devices: {:#?}", client.get_devices().await?.collect::<Vec<_>>());
# Ok(())
# }
```

If you want to discover device servers on the local network, you can do that via the `discovery::DiscoveryClient` struct:

```no_run
# #[tokio::main]
# async fn main() -> eyre::Result<()> {
use ascom_alpaca::discovery::DiscoveryClient;
use ascom_alpaca::Client;
use futures::prelude::*;

// This holds configuration for the discovery client.
// You can customize prior to binding if you want.
let discovery_client = DiscoveryClient::new();
// This results in a discovery client bound to a local socket.
// It's intentionally split out into a separate API step to encourage reuse,
// for example so that user could click "Refresh devices" button in the UI
// and the application wouldn't have to re-bind the socket every time.
let mut bound_client = discovery_client.bind().await?;
// Now you can discover devices on the local networks.
bound_client.discover_addrs()
    // create a `Client` for each discovered address
    .map(|addr| Ok(Client::new_from_addr(addr)))
    .try_for_each(|client| async move {
        /* ...do something with devices via each client... */
        Ok::<_, eyre::Error>(())
    })
    .await?;
# Ok(())
# }
```

Or, if you just want to list all available devices and don't care about per-server information or errors:

```no_run
# #[tokio::main]
# async fn main() -> eyre::Result<()> {
# use ascom_alpaca::discovery::DiscoveryClient;
# use ascom_alpaca::Client;
# use futures::prelude::*;
# let mut bound_client = DiscoveryClient::new().bind().await?;
bound_client.discover_devices()
    .for_each(|device| async move {
        /* ...do something with each device... */
    })
    .await;
# Ok(())
# }
```

Keep in mind that discovery is a UDP-based protocol, so it's not guaranteed to be reliable.

Also, same device server can be discovered multiple times if it's available on multiple network interfaces.
While it's not possible to reliably deduplicate servers, you can deduplicate devices by storing them in something like [`HashSet`](::std::collections::HashSet)
or in the same [`Devices`](crate::api::Devices) struct that is used for registering arbitrary devices on the server:

```no_run
# #[tokio::main]
# async fn main() -> eyre::Result<()> {
use ascom_alpaca::api::{Camera, Devices};
use ascom_alpaca::discovery::DiscoveryClient;
use ascom_alpaca::Client;
use futures::prelude::*;

let devices =
    DiscoveryClient::new()
    .bind()
    .await?
    .discover_devices()
    .collect::<Devices>()
    .await;

// Now you can iterate over all the discovered devices via `iter_all`:
for (typed_device, index_within_category) in devices.iter_all() {
    println!("Discovered device: {typed_device:#?} (index: {index_within_category})");
}

// ...or over devices in a specific category via `iter<dyn Trait>`:
for camera in devices.iter::<dyn Camera>() {
    println!("Discovered camera: {camera:#?}");
}
# Ok(())
# }
```

**Examples:**

- [`examples/discover.rs`](https://github.com/RReverser/ascom-alpaca-rs/blob/main/examples/discover.rs):
  A simple discovery example listing all the found servers and devices.
- [`examples/camera-client.rs`](https://github.com/RReverser/ascom-alpaca-rs/blob/main/examples/camera-client.rs):
  A cross-platform GUI example showing a live preview stream from discovered Alpaca cameras.

  Includes support for colour, monochrome and Bayer sensors with automatic colour conversion for the preview.

### Logging and tracing

This crate uses [`tracing`](https://crates.io/crates/tracing) framework for logging spans and events, integrating with the Alpaca `ClientID`, `ClientTransactionID` and `ServerTransactionID` fields.

You can enable logging in your app by using any of the [subscriber crates](https://crates.io/crates/tracing#ecosystem).

For example, [`tracing_subscriber::fmt`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html) will log all the events to stderr depending on the `RUST_LOG` environment variable:

```no_run
tracing_subscriber::fmt::init();
```

## Testing

Since this is a library for communicating to networked devices, it should be tested against real devices at a higher level.

In particular, if you're implementing an Alpaca device, make sure to run [ConformU](https://github.com/ASCOMInitiative/ConformU) - ASCOM's official conformance checker - against your device server.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE-2.0](LICENSE-APACHE-2.0))
- MIT license ([LICENSE-MIT](LICENSE-MIT))
*/
#![cfg_attr(
    all(doc, feature = "nightly"),
    feature(doc_auto_cfg, async_fn_in_trait),
    allow(incomplete_features)
)]
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::as_conversions,
    clippy::clone_on_ref_ptr,
    clippy::default_numeric_fallback,
    clippy::format_push_string,
    clippy::if_then_some_else_none,
    clippy::map_err_ignore,
    clippy::panic_in_result_fn,
    clippy::single_char_lifetime_names,
    clippy::str_to_string,
    clippy::string_to_string,
    clippy::unwrap_used,
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    // clippy::cargo,
    noop_method_call,
    single_use_lifetimes,
    unreachable_pub,
    // unsafe_code,
    unused_lifetimes,
    unused_macro_rules,
    unused_qualifications,
    unused_results,
    unused_tuple_struct_fields
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::return_self_not_must_use,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::redundant_pub_crate,
    clippy::single_match_else,
    clippy::type_repetition_in_bounds,
    clippy::let_underscore_untyped,
    clippy::struct_excessive_bools
)]

pub(crate) mod macros;

pub mod api;

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "server")]
mod server;

#[cfg(feature = "server")]
mod either;

pub mod discovery;
mod errors;
mod response;

pub use api::Devices;
#[cfg(feature = "client")]
pub use client::Client;
pub use errors::{ASCOMError, ASCOMErrorCode, ASCOMResult};
#[cfg(feature = "server")]
pub use server::{BoundServer, Server};

#[cfg(test)]
#[ctor::ctor]
fn prepare_test_env() {
    use tracing_subscriber::prelude::*;

    std::env::set_var("RUST_BACKTRACE", "full");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_target("ascom_alpaca", tracing::Level::TRACE),
        )
        .with(tracing_forest::ForestLayer::new(
            tracing_forest::printer::TestCapturePrinter::new(),
            tracing_forest::tag::NoTag,
        ))
        .with(tracing_error::ErrorLayer::default())
        .init();

    color_eyre::config::HookBuilder::default()
        .add_frame_filter(Box::new(|frames| {
            frames.retain(|frame| {
                frame.filename.as_ref().map_or(false, |filename| {
                    // Only keep our own files in the backtrace to reduce noise.
                    filename.starts_with(env!("CARGO_MANIFEST_DIR"))
                })
            });
        }))
        .install()
        .expect("Failed to install color_eyre");
}
