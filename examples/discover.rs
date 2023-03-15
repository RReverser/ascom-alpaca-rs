use futures::TryStreamExt;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ascom_alpaca_rs::discovery::discover(Duration::from_secs(3))
        .try_for_each(|addr| async move {
            println!("Found device at {}", addr);
            Ok(())
        })
        .await
}
