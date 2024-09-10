use ascom_alpaca::api::Camera;
use ascom_alpaca::test_utils::OmniSim;
use criterion::{criterion_group, criterion_main, Criterion};
use eyre::ContextCompat;
use std::time::Duration;

fn download_image_array(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let test_env = runtime
        .block_on(OmniSim::acquire())
        .expect("Failed to acquire test environment");

    c.bench_function("download_image_array", move |b| {
        let camera = runtime
            .block_on(async {
                let camera = test_env
                    .devices()
                    .iter::<dyn Camera>()
                    .next()
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

        b.iter_with_large_drop(|| {
            runtime.block_on(camera.image_array()).unwrap();
        });
    });
}

criterion_group!(benches, download_image_array);
criterion_main!(benches);
