# ascom-alpaca-rs

This is a Rust implementation of the [ASCOM Alpaca API](https://ascom-standards.org/api/).

It implements main Alpaca API clients and servers, as well as transparent support for auto-discovery mechanism and `ImageBytes` encoding for camera images.

## Usage

### Compilation features

This crate defines two sets of compilation features that help to keep binary size & compilation speed in check by opting into only the features you need.

First set is along the client-server axis:

- `client`: Enables client-side access to Alpaca-capable devices.
- `server`: Allows to expose your own devices as Alpaca servers.

The second set of features is based on the device type and enables the corresponding trait:

- `all-devices`: Enables all of the below. Not recommended unless you're building a universal astronomy application.
- `camera`: Enables support for cameras via the `Camera` trait.
- `covercalibrator`: Enables `CoverCalibrator`.
- `dome`: Enables `Dome`.
- `filterwheel`: Enables `FilterWheel`.
- `focuser`: Enables `Focuser`.
- `observingconditions`: Enables `ObservingConditions`.
- `rotator`: Enables `Rotator`.
- `switch`: Enables `Switch`.
- `telescope`: Enables `Telescope`.

Once you decided on the features you need, you can add this crate to your `Cargo.toml`. For example, if I'm implementing an Alpaca camera driver, I'd add the following to my `Cargo.toml`:

```toml
[dependencies]
ascom-alpaca = { version = "0.1", features = ["client", "camera"] }
```

### Device methods

All the device type trait methods are async and correspond to the [ASCOM Alpaca API](https://ascom-standards.org/api/). They all returns `ASCOMResult<...>` which is an alias for `Result<..., ASCOMError>`, where `ASCOMError` represents ASCOM error codes and messages.

All those traits additionally inherit from a special `Device` supertrait. It includes "ASCOM Methods Common To All Devices" from the Alpaca API, as well as few custom metadata methods used for the device registration:

- `fn static_name(&self) -> &str`: Returns the static device name. Might differ from the async `name` method result.
- `fn unique_id(&self) -> &str`: Returns globally-unique device ID.

### Implementing a device server

Since async traits are not yet natively supported on stable Rust, the traits are implemented using the [async_trait](https://crates.io/crates/async-trait) crate. Other than that, you should implement trait with all the Alpaca methods as usual:

```rust
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

Once implemented, you can create a server, register your device(s), and start listening:

```rust
let mut server = Server {
    // helper macro to populate server information from your own Cargo.toml
    info: CargoServerInfo!(),
    ..Default::default()
};

// By default, the server will listen on [::] with a randomly assigned port.
// You can change that by modifying the `listen_addr` field:
server.listen_addr.set_port(8000);

let my_camera = MyCamera { /* ... */ };
server.devices.register(my_camera);

server.start().await
```

This will start both the main Alpaca server as well as an auto-discovery responder.

See `examples/camera-server.rs` for a complete example that implements Alpaca `Camera` server for a webcam.

### Accessing devices from a client

If you know address of the device server you want to access, you can access it directly via `Client` struct:

```rust
let client = Client::new("http://localhost:8000")?;

// `get_server_info` returns high-level metadata of the server.
println!("Server info: {:#?}", client.get_server_info().await?);

// `get_devices` returns an iterator over all the devices registered on the server.
// Each is represented as a `TypedDevice` tagged enum encompassing all the device types as corresponding trait objects.
// You can either match on them to select the devices you're interested in, or, say, just print all of them:
println!("Devices: {:#?}", client.get_devices().await?.collect::<Vec<_>>());
```

If you want to discover device servers on the local network, you can do that via the `discovery::DiscoveryClient` struct:

```rust
// This holds configuration for the discovery client.
// You can customize prior to binding if you want.
let discovery_client = DiscoveryClient::new()?;
// This results in a discovery client bound to a local socket.
// It's intentionally split out into a separate API step to encourage reuse,
// for example so that user could click "Refresh devices" button in the UI
// and not have to re-bind the socket every time.
let bound_client = discovery_client.bind().await?;
// Now you can discover devices on the local networks.
bound_client.discover_addrs()
    // create a `Client` for each discovered address
    .map(Client::new_from_addr)
    .try_for_each(|client| async move {
        /* ... */
        Ok(())
    })
    .await?;
```

You can find a simple discovery example in `examples/discover.rs` and a cross-platform GUI client example for cameras in `examples/camera-client.rs`.

### Tracing

This crate uses [tracing](https://crates.io/crates/tracing) framework for logging spans and events, integrating with the Alpaca `ClientID`, `ClientTransactionID` and `ServerTransactionID` fields.

You can enable logging in your app by using any of the [subscriber crates](https://crates.io/crates/tracing#ecosystem).

For example, [`tracing-subscriber::fmt`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html) will log all the events to stderr depending on the `RUST_LOG` environment variable:

```rust
tracing_subscriber::fmt::init();
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))
