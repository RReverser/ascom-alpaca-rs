use futures::TryStreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ascom_alpaca_rs::discovery::DiscoveryClient::new()
        .discover()
        .try_for_each(|addr| async move {
            println!("Found device at {}", addr);
            Ok(())
        })
        .await
}
