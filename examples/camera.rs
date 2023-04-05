use ascom_alpaca::api::{Camera, SensorTypeResponse, TypedDevice};
use ascom_alpaca::discovery::DiscoveryClient;
use ascom_alpaca::Client;
use eframe::egui::{self, Ui};
use eframe::epaint::{Color32, ColorImage, TextureHandle, Vec2};
use futures::{Future, FutureExt, TryStreamExt};
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "ascom-alpaca-rs camera demo",
        native_options,
        Box::new(|cc| {
            Box::new(StateCtx {
                state: Default::default(),
                ctx: cc.egui_ctx.clone(),
            })
        }),
    )?;
    Ok(())
}

enum State {
    Init,
    Discovering(ChildTask),
    Discovered(Vec<Arc<dyn Camera>>),
    Connecting(ChildTask),
    Connected {
        camera_name: String,
        rx: tokio::sync::mpsc::Receiver<anyhow::Result<ColorImage>>,
        img: Option<TextureHandle>,
        fps_counter: FpsCounter,
        exposure_range: std::ops::RangeInclusive<f64>,
        capture_state: Arc<CaptureState>,
        image_loop: JoinHandle<()>, // not `ChildTask` because it has its own cancellation mechanism
    },
    Error(String),
}

impl Default for State {
    fn default() -> Self {
        Self::Init
    }
}

#[derive(Clone)]
struct StateCtx {
    state: Arc<Mutex<State>>,
    ctx: egui::Context,
}

impl StateCtx {
    fn lock(&self) -> StateCtxGuard {
        StateCtxGuard {
            state_ctx: self,
            state: self.state.lock().unwrap(),
            ctx: &self.ctx,
        }
    }
}

struct ChildTask(JoinHandle<()>);

impl Drop for ChildTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

struct StateCtxGuard<'a> {
    state_ctx: &'a StateCtx,
    state: MutexGuard<'a, State>,
    ctx: &'a egui::Context,
}

impl<'a> StateCtxGuard<'a> {
    fn set_state(&mut self, new_state: State) {
        *self.state = new_state;
        self.ctx.request_repaint();
    }

    fn set_error(&mut self, err: impl std::fmt::Display) {
        self.set_state(State::Error(format!("Error: {err:#}")));
    }

    fn spawn(
        &mut self,
        as_new_state: impl FnOnce(ChildTask) -> State,
        update: impl Future<Output = anyhow::Result<State>> + Send + 'static,
    ) {
        let state_ctx = self.state_ctx.clone();
        self.set_state(as_new_state(ChildTask(tokio::spawn(async move {
            let result = update.await;
            let mut state_ctx = state_ctx.lock();
            match result {
                Ok(state) => state_ctx.set_state(state),
                Err(err) => state_ctx.set_error(err),
            }
        }))));
    }

    fn try_update(&mut self, ui: &mut Ui) -> anyhow::Result<()> {
        match &mut *self.state {
            State::Init => {
                self.spawn(State::Discovering, async move {
                    let cameras = DiscoveryClient::new()
                        .discover_addrs()
                        .and_then(
                            |addr| async move { Client::new_from_addr(addr)?.get_devices().await },
                        )
                        .try_fold(Vec::new(), |mut cameras, new_devices| async move {
                            cameras.extend(new_devices.filter_map(|device| match device {
                                TypedDevice::Camera(camera) => Some(camera),
                                #[allow(unreachable_patterns)]
                                _ => None,
                            }));
                            Ok(cameras)
                        })
                        .await?;

                    Ok::<_, anyhow::Error>(State::Discovered(cameras))
                });
            }
            State::Discovering(_task) => {
                ui.label("Discovering cameras...");
            }
            State::Discovered(cameras) => {
                ui.label("Discovered cameras:");

                if let Some(clicked_index) = cameras
                    .iter()
                    .position(|camera| ui.button(camera.static_name()).clicked())
                {
                    let camera = cameras.swap_remove(clicked_index);
                    let ctx = self.ctx.clone();
                    self.spawn(State::Connecting, async move {
                        camera.set_connected(true).await?;
                        let camera_name = camera.name().await?;
                        let exposure_min = camera.exposure_min().await?;
                        let exposure_max = camera.exposure_max().await?;
                        let (tx, rx) = tokio::sync::mpsc::channel(1);
                        let capture_state = Arc::new(CaptureState {
                            duration_sec: AtomicF64::new(exposure_min),
                            params_change: Notify::new(),
                            tx,
                            sensor_type: camera.sensor_type().await?,
                            camera,
                            ctx,
                        });
                        let image_loop = {
                            let capture_state = Arc::clone(&capture_state);
                            tokio::spawn(async move { capture_state.start_capture_loop().await })
                        };
                        Ok(State::Connected {
                            capture_state,
                            camera_name,
                            image_loop,
                            rx,
                            img: None,
                            fps_counter: FpsCounter::new(10),
                            exposure_range: exposure_min..=exposure_max,
                        })
                    });
                }
                if ui.button("↻ Refresh").clicked() {
                    self.set_state(State::Init);
                }
                // });
            }
            State::Connecting(_task) => {
                ui.label("Connecting to camera...");
            }
            State::Connected {
                capture_state,
                camera_name,
                rx,
                img,
                fps_counter,
                exposure_range,
                image_loop,
            } => {
                ui.label(format!("Connected to camera: {}", camera_name));
                let disconnect_btn = ui.button("⏹ Disconnect");
                let mut duration_sec = capture_state.get_duration_sec();
                if ui
                    .add(
                        egui::Slider::new(&mut duration_sec, exposure_range.clone())
                            .logarithmic(true)
                            .text("Exposure (sec)"),
                    )
                    .changed()
                {
                    capture_state.set_duration_sec(duration_sec);
                    fps_counter.reset();
                }
                if let Ok(new_img) = rx.try_recv() {
                    fps_counter.tick();
                    *img = Some(ui.ctx().load_texture("img", new_img?, Default::default()));
                }
                match &*img {
                    Some(img) => {
                        ui.label(format!(
                            "Frame #{}. Rendering at {:.1} fps vs capture set to {:.1}",
                            fps_counter.total_count,
                            fps_counter.rate(),
                            1.0 / duration_sec
                        ));
                        let available_size = ui.available_size();
                        let mut img_size = Vec2::from(img.size().map(|x| x as f32));
                        // Fit the image to the available space while preserving aspect ratio.
                        img_size *= (available_size / img_size).min_elem();
                        ui.image(img, img_size)
                    }
                    None => ui.label("Starting capture stream..."),
                };
                if let Some(result) = image_loop.now_or_never() {
                    // propagate panic from the image loop
                    result?;
                }
                if disconnect_btn.clicked() {
                    self.set_state(State::Init);
                }
            }
            State::Error(err) => {
                ui.colored_label(Color32::RED, err);
                if ui.button("Restart").clicked() {
                    self.set_state(State::Init);
                }
            }
        }
        Ok(())
    }
}

struct CaptureState {
    duration_sec: atomic::Atomic<f64>,
    params_change: Notify,
    tx: tokio::sync::mpsc::Sender<anyhow::Result<ColorImage>>,
    camera: Arc<dyn Camera>,
    sensor_type: SensorTypeResponse,
    ctx: egui::Context,
}

impl CaptureState {
    fn get_duration_sec(&self) -> f64 {
        self.duration_sec.load(Ordering::Relaxed)
    }

    fn set_duration_sec(&self, duration_sec: f64) {
        self.duration_sec.store(duration_sec, Ordering::Relaxed);
        // Abort current exposure.
        self.params_change.notify_waiters();
    }

    async fn start_capture_loop(&self) {
        while self.capture_image().await {}
        // Channel is closed, cleanup.
        if let Err(err) = self.camera.set_connected(false).await {
            tracing::warn!(%err, "Failed to disconnect from the camera");
        }
    }

    async fn capture_image(&self) -> bool {
        tokio::select! {
            _ = self.tx.closed() => {
                // the receiver was dropped due to app state change
                // gracefully abort the exposure and stop the loop
                let _ = self.camera.abort_exposure().await;
                false
            }
            _ = self.params_change.notified() => {
                // the exposure parameters were changed
                // gracefully abort the exposure and continue the loop
                let _ = self.camera.abort_exposure().await;
                true
            }
            result = self.capture_image_without_cancellation() => {
                if self.tx.send(result).await.is_err() {
                    // couldn't send as the received was dropped, stop the loop
                    return false;
                }
                self.ctx.request_repaint();
                true
            }
        }
    }

    async fn capture_image_without_cancellation(&self) -> Result<ColorImage, anyhow::Error> {
        let duration_sec = self.get_duration_sec();
        self.camera.start_exposure(duration_sec, true).await?;
        while !self.camera.image_ready().await? {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let mut raw_img = self.camera.image_array().await?;
        let (width, height, depth) = raw_img.data.dim();
        // Convert from width*height*depth encoding layout to height*width*depth graphics layout.
        raw_img.data.swap_axes(0, 1);
        let mut min = i32::MAX;
        let mut max = i32::MIN;
        for &x in &raw_img.data {
            min = min.min(x);
            max = max.max(x);
        }
        let mut diff = i64::from(max - min);
        if diff == 0 {
            diff = 1;
        }
        let stretched_iter = raw_img.data.iter().map(|&x| {
            // Stretch the image.
            (i64::from(x - min) * i64::from(u8::MAX) / diff)
                .try_into()
                .unwrap()
        });
        let rgb_buf: Vec<u8> = match self.sensor_type {
            SensorTypeResponse::Color => {
                anyhow::ensure!(depth == 3, "Expected 3 channels for color image");
                stretched_iter.collect()
            }
            SensorTypeResponse::Monochrome => {
                anyhow::ensure!(depth == 1, "Expected 1 channel for monochrome image");
                stretched_iter
                    // Repeat each gray pixel 3 times to make it RGB.
                    .flat_map(|color| std::iter::repeat(color).take(3))
                    .collect()
            }
            SensorTypeResponse::RGGB => {
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

impl eframe::App for StateCtx {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut state_ctx = self.lock();
            if let Err(err) = &mut state_ctx.try_update(ui) {
                state_ctx.set_error(err);
            }
        });
    }
}