use ascom_alpaca::api::{Camera, SensorType, TypedDevice};
use ascom_alpaca::discovery::DiscoveryClient;
use ascom_alpaca::Client;
use dioxus::prelude::*;
use eframe::egui::{self, Ui};
use eframe::epaint::{Color32, ColorImage, TextureHandle, Vec2};
use futures::{Future, FutureExt, StreamExt, TryStreamExt};
use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

type AtomicF64 = atomic::Atomic<f64>;

struct FpsCounter {
    total_count: u32,
    timings: VecDeque<Instant>,
}

impl FpsCounter {
    pub fn new(capacity: usize) -> Self {
        Self {
            total_count: 0,
            timings: VecDeque::with_capacity(capacity),
        }
    }

    pub fn tick(&mut self) {
        if self.timings.len() == self.timings.capacity() {
            self.timings.pop_front();
        }
        self.timings.push_back(Instant::now());
        self.total_count += 1;
    }

    pub fn rate(&self) -> f64 {
        let frame_count = self.timings.len().saturating_sub(1);
        if frame_count == 0 {
            return 0.0;
        }
        let oldest = *self.timings.front().unwrap();
        let newest = *self.timings.back().unwrap();
        let duration = newest - oldest;
        frame_count as f64 / duration.as_secs_f64()
    }

    pub fn reset(&mut self) {
        self.timings.clear();
    }
}

fn render_err(cx: Scope, error: impl std::fmt::Display) -> Element {
    cx.render(rsx! {
        h1 { style: "color: red", "Error: {error}" }
    })
}

#[derive(Clone)]
struct ComparableCamera(Arc<dyn Camera>);

impl PartialEq for ComparableCamera {
    fn eq(&self, other: &Self) -> bool {
        self.0.unique_id() == other.0.unique_id()
    }
}

#[derive(Debug)]
struct CameraInfo {
    name: String,
    exposure_min: f64,
    exposure_max: f64,
    sensor_type: SensorType,
}

fn app(cx: Scope) -> Element {
    // todo: just use `discovering.state()` when it's implemented upstream
    let is_discovering = use_state(cx, || false);

    let discovering = {
        let is_discovering = is_discovering.clone();

        use_future!(cx, || async move {
            is_discovering.set(true);

            let cameras = DiscoveryClient::new()
                .discover_addrs()
                .and_then(|addr| async move { Client::new_from_addr(addr)?.get_devices().await })
                .try_fold(Vec::new(), |mut cameras, new_devices| async move {
                    cameras.extend(new_devices.filter_map(|device| match device {
                        TypedDevice::Camera(camera) => Some(camera),
                        #[allow(unreachable_patterns)]
                        _ => None,
                    }));
                    Ok(cameras)
                })
                .await?;

            is_discovering.set(false);

            Ok::<_, anyhow::Error>(cameras)
        })
    };

    let cameras = match discovering.value().filter(|_| !is_discovering) {
        Some(Ok(cameras)) => cameras,
        Some(Err(err)) => return render_err(cx, err),
        None => return cx.render(rsx!(h1 { "Discovering..." })),
    };

    let camera = use_state(cx, || None);

    let is_connecting = use_state(cx, || false);

    let connecting = {
        let is_connecting = is_connecting.clone();

        use_future!(cx, |camera| async move {
            let camera = match camera.get() {
                Some(ComparableCamera(camera)) => camera,
                None => return Ok::<_, anyhow::Error>(None),
            };

            is_connecting.set(true);

            camera.set_connected(true).await?;
            let camera_info = CameraInfo {
                name: camera.name().await?,
                exposure_min: camera.exposure_min().await?,
                exposure_max: camera.exposure_max().await?,
                sensor_type: camera.sensor_type().await?,
            };

            is_connecting.set(false);

            Ok::<_, anyhow::Error>(Some(camera_info))
        })
    };

    let camera_info = match connecting.value().filter(|_| !is_connecting) {
        Some(Ok(None)) => {
            return cx.render(rsx!(
                h1 { "Discovered cameras:" },
                cameras.iter().map(|camera_| rsx!(button {
                    key: "{camera_.unique_id()}",
                    onclick: move |_| {
                        camera.set(Some(ComparableCamera(Arc::clone(camera_))));
                    },
                    camera_.static_name()
                })),
                button {
                    onclick: move |_| {
                        discovering.restart();
                    },
                    "↻ Refresh"
                }
            ))
        }
        Some(Ok(Some(camera_info))) => camera_info,
        Some(Err(err)) => return render_err(cx, err),
        None => return cx.render(rsx!(h1 { "Connecting..." })),
    };

    let capture_params = use_state(cx, || CaptureParams {
        duration: Duration::from_secs(1),
        camera: camera.get().as_ref().unwrap().clone(),
        sensor_type: camera_info.sensor_type,
    });

    let latest_image = use_state(cx, || None);

    let capture_loop = {
        let capture_params = capture_params.clone();
        let latest_image = latest_image.clone();

        use_coroutine(cx, |rx| async move {
            capture_params.start_capture_loop(latest_image, rx).await
        })
    };

    let old_capture_params = use_state(cx, || capture_params.current());

    if old_capture_params.get() != capture_params.get_rc() {
        capture_loop.send(());
        old_capture_params.set(capture_params.current());
    }

    cx.render(rsx!(
        h1 { "Connected to {camera_info.name}" }
        h2 { "Exposure range: {camera_info.exposure_min} - {camera_info.exposure_max}" }
        h2 { "Sensor type: {camera_info.sensor_type:?}" }
        button {
            onclick: move |_| {
                camera.set(None);
            },
            "Disconnect"
        }
        if let Some(latest_image) = latest_image.as_ref() {
            // let url = format!("data:image/png;base64,{}", base64::encode(&latest_image.to_png()));
            // rsx!(img {
            //     src: "{url}",
            //     style: "max-width: 100%; max-height: 100%;"
            // })
            rsx!("{latest_image.pixels.len()}")
        }
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    dioxus_desktop::launch(app);

    Ok(())
}

#[derive(PartialEq, Clone)]
struct CaptureParams {
    duration: Duration,
    sensor_type: SensorType,
    camera: ComparableCamera,
}

impl CaptureParams {
    async fn start_capture_loop(
        &self,
        latest_image: UseState<Option<ColorImage>>,
        mut abort_rx: UnboundedReceiver<()>,
    ) {
        loop {
            tokio::select! {
                abort = abort_rx.next() => {
                    let is_end = abort.is_none();
                    tracing::warn!(is_end, "Aborting exposure");
                    let _ignore_err = self.camera.0.abort_exposure().await;
                    if is_end {
                        break;
                    }
                }
                res = self.capture_image_without_cancellation() => {
                    match res {
                        Ok(image) => {
                            latest_image.set(Some(image));
                        }
                        Err(err) => {
                            tracing::warn!(%err, "Failed to capture an image");
                        }
                    }
                }
            }
        }
        // Channel is closed, cleanup.
        if let Err(err) = self.camera.0.set_connected(false).await {
            tracing::warn!(%err, "Failed to disconnect from the camera");
        }
    }

    async fn capture_image_without_cancellation(&self) -> Result<ColorImage, anyhow::Error> {
        let camera = &self.camera.0;
        let duration_sec = self.duration.as_secs_f64();
        camera.start_exposure(duration_sec, true).await?;
        while !camera.image_ready().await? {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let raw_img = camera.image_array().await?;
        let (width, height, depth) = raw_img.dim();
        // Convert from width*height*depth encoding layout to height*width*depth graphics layout.
        let mut data = raw_img.view();
        data.swap_axes(0, 1);
        let mut min = i32::MAX;
        let mut max = i32::MIN;
        for &x in data {
            min = min.min(x);
            max = max.max(x);
        }
        let mut diff = i64::from(max - min);
        if diff == 0 {
            diff = 1;
        }
        let stretched_iter = raw_img.iter().map(|&x| {
            // Stretch the image.
            (i64::from(x - min) * i64::from(u8::MAX) / diff)
                .try_into()
                .unwrap()
        });
        let rgb_buf: Vec<u8> = match self.sensor_type {
            SensorType::Color => {
                anyhow::ensure!(depth == 3, "Expected 3 channels for color image");
                stretched_iter.collect()
            }
            SensorType::Monochrome => {
                anyhow::ensure!(depth == 1, "Expected 1 channel for monochrome image");
                stretched_iter
                    // Repeat each gray pixel 3 times to make it RGB.
                    .flat_map(|color| std::iter::repeat(color).take(3))
                    .collect()
            }
            SensorType::RGGB => {
                struct ReadIter<I>(I);

                impl<I: ExactSizeIterator<Item = u8>> std::io::Read for ReadIter<I> {
                    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                        let merged_iter = buf.iter_mut().zip(&mut self.0);
                        let len = merged_iter.len();
                        for (dst, src) in merged_iter {
                            *dst = src;
                        }
                        Ok(len)
                    }
                }

                anyhow::ensure!(depth == 1, "Expected 1 channel for RGGB image");

                let mut rgb_buf = vec![0; width * height * 3];

                bayer::demosaic::linear::run(
                    &mut ReadIter(stretched_iter),
                    bayer::BayerDepth::Depth8,
                    bayer::CFA::RGGB,
                    &mut bayer::RasterMut::new(
                        width,
                        height,
                        bayer::RasterDepth::Depth8,
                        &mut rgb_buf,
                    ),
                )?;

                rgb_buf
            }
            other => {
                anyhow::bail!("Unsupported sensor type: {:?}", other)
            }
        };
        Ok(ColorImage::from_rgb([width, height], &rgb_buf))
    }
}
