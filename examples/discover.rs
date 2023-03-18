#[cfg(feature = "client")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use futures::TryStreamExt;

    let mut client = ascom_alpaca_rs::client::DiscoveryClient::new();
    client.include_ipv6 = true;
    client
        .discover_addrs()
        .try_for_each(|addr| async move {
            println!("Found Alpaca server at {addr}");
            Ok(())
        })
        .await
}

#[cfg(not(feature = "client"))]
fn main() {
    println!("This example requires the `client` feature");
}
