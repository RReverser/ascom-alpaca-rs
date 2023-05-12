use ascom_alpaca::discovery::DiscoveryClient;
use ascom_alpaca::Client;
use futures::{StreamExt, TryStreamExt};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    println!("Searching...");

    DiscoveryClient::new()
        .bind()
        .await?
        .discover_addrs()
        .map(Client::new_from_addr)
        .try_for_each(|client| async move {
            let server_info = client.get_server_info().await?;
            println!("Server info: {server_info:#?}");
            let devices = client.get_devices().await?.collect::<Vec<_>>();
            println!("Devices: {devices:#?}");
            Ok::<_, eyre::Error>(())
        })
        .await?;

    println!("Discovery completed");

    Ok(())
}
