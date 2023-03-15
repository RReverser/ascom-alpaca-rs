use futures::TryStreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = ascom_alpaca_rs::discovery::DiscoveryClient::new();
    client.include_ipv6 = true;
    client
        .discover_addrs()
        .try_for_each(|addr| async move {
            println!("Found Alpaca server at {addr}");
            Ok(())
        })
        .await
}
