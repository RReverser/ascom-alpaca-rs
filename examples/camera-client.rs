use ascom_alpaca::api::{Camera, ImageArray, SensorType as AlpacaSensorType, TypedDevice};
use ascom_alpaca::discovery::{BoundDiscoveryClient, DiscoveryClient};
use ascom_alpaca::{ASCOMErrorCode, ASCOMResult};
use eframe::egui::{self, TextureOptions, Ui};
use eframe::epaint::{Color32, ColorImage, TextureHandle, Vec2};
use eyre::Context;
use futures::{Future, FutureExt, StreamExt, TryFutureExt};
use std::collections::HashSet;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

enum State {
    Init,
    Discovering(ChildTask),
    Discovered(HashSet<Arc<dyn Camera>>),
    Connecting(ChildTask),
    Connected {
        camera_name: String,
        rx: tokio::sync::mpsc::Receiver<eyre::Result<ColorImage>>,
        frame_num: u32,
        img: Option<TextureHandle>,
        exposure_range: RangeInclusive<f64>,
        gain_mode: GainMode,
        params_tx: tokio::sync::watch::Sender<CaptureParams>,
        image_loop: JoinHandle<()>, /* not `ChildTask` because it has its own cancellation mechanism */
    },
    Error(String),
}

impl State {
    fn error(err: eyre::Error) -> Self {
        Self::Error(format!("{err:#}"))
    }
}

enum GainMode {
    Range(RangeInclusive<i32>),
    List(Vec<String>),
    None,
}

struct ChildTask(JoinHandle<State>);

impl Drop for ChildTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn if_implemented<T>(res: ASCOMResult<T>) -> ASCOMResult<Option<T>> {
    match res {
        Err(err) if err.code == ASCOMErrorCode::NOT_IMPLEMENTED => Ok(None),
        _ => res.map(Some),
    }
}

struct StateCtx {
    discovery_client: Arc<tokio::sync::Mutex<Option<BoundDiscoveryClient>>>,
    state: State,
    ctx: egui::Context,
}

impl StateCtx {
    fn set_state(&mut self, new_state: State) {
        self.state = new_state;
        self.ctx.request_repaint();
    }

    fn spawn(
        &mut self,
        as_new_state: impl FnOnce(ChildTask) -> State,
        update: impl Future<Output = eyre::Result<State>> + Send + 'static,
    ) {
        let ctx = self.ctx.clone();
        self.set_state(as_new_state(ChildTask(tokio::spawn(async move {
            let result = update.await;
            ctx.request_repaint();
            match result {
                Ok(state) => state,
                Err(err) => State::error(err),
            }
        }))));
    }

    fn try_update(&mut self, ui: &mut Ui) -> eyre::Result<()> {
        match &mut self.state {
            State::Init => {
                let discovery_client = Arc::clone(&self.discovery_client);

                self.spawn(State::Discovering, async move {
                    let mut discovery_client = discovery_client.lock().await;

                    let discovery_client = match &mut *discovery_client {
                        Some(discovery_client) => discovery_client,
                        None => {
                            *discovery_client = Some(DiscoveryClient::new().bind().await?);
                            discovery_client.as_mut().unwrap()
                        }
                    };

                    let cameras = discovery_client
                        .discover_devices()
                        .filter_map(|device| async move {
                            match device {
                                TypedDevice::Camera(camera) => Some(camera),
                                #[allow(unreachable_patterns)]
                                _ => None,
                            }
                        })
                        .collect::<HashSet<_>>()
                        .await;

                    Ok::<_, eyre::Error>(State::Discovered(cameras))
                });
            }
            State::Discovering(ChildTask(task)) => {
                ui.label("Discovering cameras...");
                if let Some(new_state) = task.now_or_never() {
                    self.set_state(new_state?);
                }
            }
            State::Discovered(cameras) => {
                ui.label("Discovered cameras:");

                if let Some(camera) = cameras
                    .iter()
                    .find(|camera| ui.button(camera.static_name()).clicked())
                {
                    let camera = Arc::clone(camera);
                    let ctx = self.ctx.clone();
                    self.spawn(State::Connecting, async move {
                        camera.set_connected(true).await?;
                        let (
                            camera_name,
                            exposure_min,
                            exposure_max,
                            can_abort_exposure,
                            max_adu,
                            sensor_type,
                            (gain_mode, gain),
                        ) = tokio::try_join!(
                            camera.name().map_err(eyre::Error::from),
                            camera.exposure_min().map_err(eyre::Error::from),
                            camera.exposure_max().map_err(eyre::Error::from),
                            camera.can_abort_exposure().map_err(eyre::Error::from),
                            async {
                                let max_adu = camera.max_adu().await?;
                                u32::try_from(max_adu).context("Max ADU is out of range")
                            },
                            async {
                                Ok(match camera.sensor_type().await? {
                                    AlpacaSensorType::Monochrome => SensorType::Monochrome,
                                    AlpacaSensorType::Color => SensorType::Color,
                                    AlpacaSensorType::RGGB => {
                                        let (offset_x, offset_y) = tokio::try_join!(
                                            camera.bayer_offset_x(),
                                            camera.bayer_offset_y()
                                        )?;
                                        SensorType::Bayer(match (offset_x, offset_y) {
                                            (0, 0) => bayer::CFA::RGGB,
                                            (1, 0) => bayer::CFA::GRBG,
                                            (0, 1) => bayer::CFA::GBRG,
                                            (1, 1) => bayer::CFA::BGGR,
                                            _ => eyre::bail!("Invalid bayer offset: ({}, {})", offset_x, offset_y),
                                        })
                                    }
                                    sensor_type => {
                                        tracing::warn!("Unsupported sensor type {sensor_type:?}, treating as monochrome");
                                        SensorType::Monochrome
                                    }
                                })
                            },
                            async {
                                let gain_mode = match if_implemented(camera.gain_min().await)? {
                                    Some(min) => GainMode::Range(min..=camera.gain_max().await?),
                                    None => match if_implemented(camera.gains().await)? {
                                        Some(list) => GainMode::List(list),
                                        None => GainMode::None,
                                    },
                                };
                                let gain = match gain_mode {
                                    GainMode::None => 0,
                                    _ => camera.gain().await?,
                                };
                                Ok((gain_mode, gain))
                            }
                        )?;
                        let (tx, rx) = tokio::sync::mpsc::channel(1);
                        let (params_tx, params_rx) = tokio::sync::watch::channel(CaptureParams {
                            duration_sec: 0.05,
                            gain,
                            dynamic_stretch: true
                        });
                        let image_loop = tokio::spawn(
                            CaptureState {
                                params_rx,
                                tx,
                                sensor_type,
                                camera,
                                ctx,
                                stored_gain: gain,
                                can_abort_exposure,
                                max_adu,
                            }
                            .start_capture_loop(),
                        );
                        Ok(State::Connected {
                            camera_name,
                            image_loop,
                            params_tx,
                            gain_mode,
                            frame_num: 0,
                            rx,
                            img: None,
                            exposure_range: exposure_min..=exposure_max,
                        })
                    });
                }
                if ui.button("↻ Refresh").clicked() {
                    self.set_state(State::Init);
                }
            }
            State::Connecting(ChildTask(task)) => {
                ui.label("Connecting to camera...");
                if let Some(new_state) = task.now_or_never() {
                    self.set_state(new_state?);
                }
            }
            State::Connected {
                params_tx,
                gain_mode,
                camera_name,
                rx,
                img,
                exposure_range,
                image_loop,
                frame_num,
            } => {
                ui.label(format!("Connected to camera: {camera_name}"));
                let disconnect_btn = ui.button("⏹ Disconnect");
                params_tx.send_if_modified(|params| {
                    let exposure_changed = ui
                        .add(
                            egui::Slider::new(&mut params.duration_sec, exposure_range.clone())
                                .logarithmic(true)
                                .text("Exposure (sec)"),
                        )
                        .changed();
                    let gain_changed = match gain_mode {
                        GainMode::List(values) => egui::ComboBox::from_label("Gain")
                            .selected_text(&values[params.gain as usize])
                            .show_ui(ui, |ui| {
                                values.iter().enumerate().any(|(i, value)| {
                                    ui.selectable_value(&mut params.gain, i as i32, value)
                                        .clicked()
                                })
                            })
                            .inner
                            .unwrap_or(false),
                        GainMode::Range(range) => ui
                            .add(egui::Slider::new(&mut params.gain, range.clone()).text("Gain"))
                            .changed(),
                        GainMode::None => false,
                    };
                    let dynamic_stretch_changed = ui
                        .checkbox(&mut params.dynamic_stretch, "Dynamic stretch")
                        .changed();
                    exposure_changed || gain_changed || dynamic_stretch_changed
                });
                if let Ok(new_img) = rx.try_recv() {
                    *frame_num += 1;
                    *img = Some(
                        ui.ctx()
                            .load_texture("img", new_img?, TextureOptions::default()),
                    );
                }
                ui.label(format!("Frame: {frame_num}"));
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

#[derive(Clone, Copy)]
struct CaptureParams {
    duration_sec: f64,
    gain: i32,
    dynamic_stretch: bool,
}

enum SensorType {
    Monochrome,
    Color,
    Bayer(bayer::CFA),
}

struct CaptureState {
    params_rx: tokio::sync::watch::Receiver<CaptureParams>,
    tx: tokio::sync::mpsc::Sender<eyre::Result<ColorImage>>,
    camera: Arc<dyn Camera>,
    sensor_type: SensorType,
    ctx: egui::Context,
    can_abort_exposure: bool,
    stored_gain: i32,
    max_adu: u32,
}

impl CaptureState {
    async fn start_capture_loop(mut self) {
        while !self.tx.is_closed() {
            if let Some(send) = self.capture_image().await.transpose() {
                if self.tx.send(send).await.is_ok() {
                    self.ctx.request_repaint();
                }
            }
        }
        // Channel is closed, cleanup.
        if let Err(err) = self.camera.set_connected(false).await {
            tracing::warn!(%err, "Failed to disconnect from the camera");
        }
    }

    async fn capture_image(&mut self) -> eyre::Result<Option<ColorImage>> {
        let gain = self.params_rx.borrow_and_update().gain;
        if gain != self.stored_gain {
            self.camera.set_gain(gain).await?;
            self.stored_gain = gain;
        }
        // a separate cheap clone to watch notifications on without borrowing from `self`
        let mut params_rx = self.params_rx.clone();

        tokio::select! {
            _ = async {
                tokio::select! {
                    // the receiver was dropped due to app state change
                    _ = self.tx.closed() => {},
                    // or the exposure params were changed
                    _ = params_rx.changed() => {},
                }
            }, if self.can_abort_exposure => {
                self.camera.abort_exposure().await?;
                Ok(None)
            }
            result = self.capture_image_without_cancellation() => {
                result.map(Some)
            }
        }
    }

    async fn capture_image_without_cancellation(&self) -> Result<ColorImage, eyre::Error> {
        let params = *self.params_rx.borrow();
        self.camera
            .start_exposure(params.duration_sec, true)
            .await?;
        tokio::time::sleep(Duration::from_secs_f64(params.duration_sec)).await;
        while !self.camera.image_ready().await? {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let raw_img = self.camera.image_array().await?;
        to_stretched_color_img(
            &self.sensor_type,
            if params.dynamic_stretch {
                raw_img
                    .iter()
                    .max()
                    .and_then(|&x| u32::try_from(x).ok())
                    .unwrap_or(1)
            } else {
                self.max_adu
            },
            &raw_img,
        )
    }
}

fn to_stretched_color_img(
    sensor_type: &SensorType,
    max_adu: u32,
    raw_img: &ImageArray,
) -> Result<ColorImage, eyre::Error> {
    let (width, height, depth) = raw_img.dim();
    let mut raw_img = raw_img.view();
    // Convert from width*height*depth encoding layout to height*width*depth graphics layout.
    raw_img.swap_axes(0, 1);
    let stretched_iter = raw_img.iter().map(|&x| {
        // clamp sub-zero values
        let x = u32::try_from(x).unwrap_or(0);
        // Stretch the image from [0; max_adu] to u8 range.
        // Use u64 as a cheap replacement for floating-point math
        // that can still fit the temporary math values.
        (u64::from(x) * u64::from(u8::MAX) / u64::from(max_adu)) as u8
    });
    let rgb_buf: Vec<u8> = match sensor_type {
        SensorType::Monochrome => {
            eyre::ensure!(
                depth == 1,
                "Expected 1 channel for monochrome image but got {}",
                depth,
            );
            stretched_iter
                // Repeat each gray pixel 3 times to make it RGB.
                .flat_map(|color| std::iter::repeat(color).take(3))
                .collect()
        }
        SensorType::Color => {
            eyre::ensure!(
                depth == 3,
                "Expected 3 channels for color image but got {}",
                depth,
            );
            stretched_iter.collect()
        }
        SensorType::Bayer(cfa) => {
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

            eyre::ensure!(
                depth == 1,
                "Expected 1 channel for RGGB image but got {}",
                depth,
            );

            let mut rgb_buf = vec![0; width * height * 3];

            bayer::demosaic::linear::run(
                &mut ReadIter(stretched_iter),
                bayer::BayerDepth::Depth8,
                *cfa,
                &mut bayer::RasterMut::new(width, height, bayer::RasterDepth::Depth8, &mut rgb_buf),
            )?;

            rgb_buf
        }
    };
    Ok(ColorImage::from_rgb([width, height], &rgb_buf))
}

impl eframe::App for StateCtx {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Err(err) = self.try_update(ui) {
                self.set_state(State::error(err));
            }
        });
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
                state: State::Init,
                ctx: cc.egui_ctx.clone(),
                discovery_client: Default::default(),
            })
        }),
    )?;
    Ok(())
}
