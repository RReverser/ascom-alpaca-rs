#[cfg(all(feature = "client", feature = "all-devices"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use ascom_alpaca::discovery::DiscoveryClient;
    use ascom_alpaca::Client;
    use futures::TryStreamExt;

    tracing_subscriber::fmt::init();

    println!("Searching...");

    DiscoveryClient::new()
        .discover_addrs()
        .try_for_each(|addr| async move {
            println!("Found Alpaca server at {addr}");
            let client = Client::new_from_addr(addr)?;
            let server_info = client.get_server_info().await?;
            println!("Server info: {server_info:#?}");
            let devices = client.get_devices().await?.collect::<Vec<_>>();
            println!("Devices: {devices:#?}");
            Ok(())
        })
        .await?;

    println!("Discovery completed");

    Ok(())
}

#[cfg(not(all(feature = "client", feature = "all-devices")))]
fn main() {
    eprintln!("This example requires the 'client' and 'all-devices' features");
}
