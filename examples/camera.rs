use ascom_alpaca::api::{Camera, Device, DeviceType, SensorTypeResponse};
use ascom_alpaca::discovery::DiscoveryClient;
use ascom_alpaca::{Client, DeviceClient};
use eframe::egui::{self, Ui};
use eframe::epaint::{Color32, ColorImage, TextureHandle, Vec2};
use futures::{Future, FutureExt, TryStreamExt};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
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
    Discovering(JoinHandle<()>),
    Discovered {
        devices: Vec<DeviceClient>,
        selected_index: Option<usize>,
    },
    Connecting(JoinHandle<()>),
    Connected {
        duration_ms: Arc<AtomicU32>,
        camera_name: String,
        image_loop: JoinHandle<()>,
        rx: tokio::sync::mpsc::Receiver<anyhow::Result<ColorImage>>,
        img: Option<TextureHandle>,
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
        as_new_state: impl FnOnce(JoinHandle<()>) -> State,
        update: impl Future<Output = anyhow::Result<State>> + Send + 'static,
    ) {
        let state_ctx = self.state_ctx.clone();
        self.set_state(as_new_state(tokio::spawn(async move {
            let result = update.await;
            let mut state_ctx = state_ctx.lock();
            match result {
                Ok(state) => state_ctx.set_state(state),
                Err(err) => state_ctx.set_error(err),
            }
        })));
    }

    fn try_update(&mut self, ui: &mut Ui) -> anyhow::Result<()> {
        match &mut *self.state {
            State::Init => {
                self.spawn(State::Discovering, async move {
                    let devices = DiscoveryClient::new()
                        .discover_addrs()
                        .and_then(
                            |addr| async move { Client::new_from_addr(addr)?.get_devices().await },
                        )
                        .try_fold(Vec::new(), |mut devices, new_devices| async move {
                            devices.extend(new_devices);
                            Ok(devices)
                        })
                        .await?;

                    Ok::<_, anyhow::Error>(State::Discovered {
                        devices,
                        selected_index: None,
                    })
                });
            }
            State::Discovering(_task) => {
                ui.label("Discovering cameras...");
            }
            State::Discovered {
                devices,
                selected_index,
            } => {
                ui.label("Discovered cameras:");
                egui::ComboBox::from_label("")
                    .selected_text(
                        selected_index
                            .map(|i| devices[i].static_name())
                            .unwrap_or("(none)"),
                    )
                    .show_ui(ui, |ui| {
                        for (i, device) in devices
                            .iter()
                            .filter(|device| device.ty() == DeviceType::Camera)
                            .enumerate()
                        {
                            ui.selectable_value(selected_index, Some(i), device.static_name());
                        }
                    });
                // ui.horizontal(|ui| {
                if ui
                    .add_enabled(selected_index.is_some(), egui::Button::new("Connect"))
                    .clicked()
                {
                    let device = devices.swap_remove(selected_index.unwrap());
                    let ctx = self.ctx.clone();
                    self.spawn(State::Connecting, async move {
                        device.set_connected(true).await?;
                        let camera_name = device.name().await?;
                        let sensor_type = device.sensor_type().await?;
                        let (tx, rx) = tokio::sync::mpsc::channel(1);
                        let duration_ms = Arc::new(AtomicU32::new(100));
                        let duration_ms_clone = Arc::clone(&duration_ms);
                        let image_loop = tokio::spawn(async move {
                            loop {
                                let result = async {
                                    let duration = Duration::from_millis(duration_ms_clone.load(Ordering::Relaxed).into());
                                    device.start_exposure(duration.as_secs_f64(), true).await?;
                                    tokio::time::sleep(duration).await;
                                    while !device.image_ready().await? {
                                        tokio::time::sleep(Duration::from_millis(50)).await;
                                    }
                                    let mut raw_img = device.image_array().await?;
                                    let (width, height, depth) = raw_img.data.dim();
                                    tracing::debug!(
                                        width,
                                        height,
                                        depth,
                                        "Got image"
                                    );
                                    // Convert from standard row-major layout used by encoding to column-major layout used by graphics.
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
                                    let rgb_buf: Vec<u8> = match sensor_type {
                                        SensorTypeResponse::Color => {
                                            anyhow::ensure!(
                                                depth == 3,
                                                "Expected 3 channels for color image"
                                            );
                                            stretched_iter.collect()
                                        }
                                        SensorTypeResponse::Monochrome => {
                                            anyhow::ensure!(
                                                depth == 1,
                                                "Expected 1 channel for monochrome image"
                                            );
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

                                            anyhow::ensure!(
                                                depth == 1,
                                                "Expected 1 channel for RGGB image"
                                            );

                                            let mut rgb_buf = vec![0; width * height * 3];

                                            bayer::demosaic::linear::run(
                                                &mut ReadIter(raw_img.data.iter().map(|&x| x as u8)),
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
                                    let color_img = ColorImage::from_rgb([width, height], &rgb_buf);
                                    Ok(color_img)
                                }
                                .await;
                                if tx.send(result).await.is_err() {
                                    break;
                                }
                                ctx.request_repaint();
                            }
                            // Channel is closed, cleanup.
                            if let Err(err) = device.set_connected(false).await {
                                tracing::warn!(%err, "Failed to disconnect from the camera");
                            }
                        });
                        Ok(State::Connected {
                            duration_ms,
                            camera_name,
                            image_loop,
                            rx,
                            img: None,
                        })
                    });
                }
                if ui.button("Refresh").clicked() {
                    self.set_state(State::Init);
                }
                // });
            }
            State::Connecting(_task) => {
                ui.label("Connecting to camera...");
            }
            State::Connected {
                duration_ms,
                camera_name,
                image_loop,
                rx,
                img,
            } => {
                ui.label(format!("Connected to camera: {}", camera_name));
                let mut duration_ms_copy = duration_ms.load(Ordering::Relaxed);
                if ui
                    .add(egui::Slider::new(&mut duration_ms_copy, 2..=1000).text("Exposure (ms)"))
                    .changed()
                {
                    duration_ms.store(duration_ms_copy, Ordering::Relaxed);
                }
                let disconnect_btn = ui.button("Disconnect");
                if let Ok(new_img) = rx.try_recv() {
                    *img = Some(ui.ctx().load_texture("img", new_img?, Default::default()));
                }
                match &*img {
                    Some(img) => {
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
