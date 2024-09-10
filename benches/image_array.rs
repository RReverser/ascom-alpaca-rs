use ascom_alpaca::api::TypedDevice;
use ascom_alpaca::Client;
use criterion::{criterion_group, criterion_main, Criterion};
use eyre::ContextCompat;
use std::time::Duration;

fn download_image_array(c: &mut Criterion) {
    c.bench_function("download_image_array", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let camera = runtime
            .block_on(async move {
                // Create client against the default Alpaca simulators port.
                let client = Client::new("http://localhost:32323/")?;
                let camera = client
                    .get_devices()
                    .await?
                    .find_map(|device| match device {
                        TypedDevice::Camera(camera) => Some(camera),
                        #[allow(unreachable_patterns)]
                        _ => None,
                    })
                    .context("No camera found")?;
                camera.set_connected(true).await?;
                camera.start_exposure(0.001, true).await?;
                loop {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    if camera.image_ready().await? {
                        break;
                    }
                }
                Ok::<_, eyre::Error>(camera)
            })
            .expect("Failed to capture a test image");

        b.iter_with_large_drop(|| runtime.block_on(camera.image_array()).unwrap());
    });
}

criterion_group!(benches, download_image_array);
criterion_main!(benches);
