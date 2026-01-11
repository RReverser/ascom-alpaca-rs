//! A cross-platform example exposing your connected webcam(s) as Alpaca `Camera`s.

#![expect(
    clippy::as_conversions,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    clippy::default_numeric_fallback
)]

use ascom_alpaca::api::camera::{CameraState, ImageArray, SensorType};
use ascom_alpaca::api::{Camera as AlpacaCamera, CargoServerInfo, Device};
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult, Server};
use async_trait::async_trait;
use eyre::ContextCompat;
use futures::future::{BoxFuture, FutureExt, Shared};
use ndarray::Array3;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{
    CameraFormat, CameraInfo, FrameFormat, RequestedFormat, RequestedFormatType, Resolution,
};
use nokhwa::{Camera, NokhwaError, nokhwa_initialize};
use parking_lot::{Mutex, RwLock};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, oneshot};
use tokio::task;

const ERR_EXPOSURE_FAILED_TO_STOP: ASCOMError = ASCOMError {
    code: ASCOMErrorCode::new_for_driver(0),
    message: Cow::Borrowed("Exposure failed to stop correctly"),
};

const ERR_EXPOSING_STATE_CHANGED_UNEXPECTEDLY: ASCOMError = ASCOMError {
    code: ASCOMErrorCode::new_for_driver(1),
    message: Cow::Borrowed(
        "Internal error: exposing state changed unexpectedly during an active exposure",
    ),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy)]
struct Size {
    x: i32,
    y: i32,
}

impl From<Resolution> for Size {
    fn from(resolution: Resolution) -> Self {
        Self {
            x: resolution.width() as _,
            y: resolution.height() as _,
        }
    }
}

impl std::ops::Add<Size> for Point {
    type Output = Self;

    fn add(self, size: Size) -> Self::Output {
        Self {
            x: self.x + size.x,
            y: self.y + size.y,
        }
    }
}

// Define comparison as "is this point more bottom-right than the other?".
impl Ord for Point {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.x.cmp(&other.x), self.y.cmp(&other.y)) {
            (Ordering::Less, _) | (_, Ordering::Less) => Ordering::Less,
            (Ordering::Equal, Ordering::Equal) => Ordering::Equal,
            (Ordering::Greater, _) | (_, Ordering::Greater) => Ordering::Greater,
        }
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
struct Subframe {
    bin: Size,
    offset: Point,
    size: Size,
}

struct StopExposure {
    want_image: bool,
}

enum ExposingState {
    Idle {
        camera: Arc<Mutex<Camera>>,
        image: Option<ImageArray>,
    },
    Exposing {
        start: Instant,
        expected_duration: Duration,
        stop_tx: Option<oneshot::Sender<StopExposure>>,
        done: Shared<BoxFuture<'static, bool>>,
    },
}

#[derive(derive_more::Debug)]
struct Webcam {
    unique_id: String,
    name: String,
    description: String,
    max_format: CameraFormat,
    subframe: RwLock<Subframe>,
    #[debug(skip)]
    exposing: Arc<RwLock<ExposingState>>,
    last_exposure_start_time: RwLock<Option<SystemTime>>,
    last_exposure_duration: Arc<RwLock<Option<Duration>>>,
    valid_bins: Vec<i32>,
}

fn convert_err(nokhwa: NokhwaError) -> ASCOMError {
    ASCOMError::new(
        ASCOMErrorCode::new_for_driver(match nokhwa {
            NokhwaError::UnitializedError => 0,
            NokhwaError::InitializeError { .. } => 1,
            NokhwaError::ShutdownError { .. } => 2,
            NokhwaError::GeneralError(_) => 3,
            NokhwaError::StructureError { .. } => 4,
            NokhwaError::OpenDeviceError(_, _) => 5,
            NokhwaError::GetPropertyError { .. } => 6,
            NokhwaError::SetPropertyError { .. } => 7,
            NokhwaError::OpenStreamError(_) => 8,
            NokhwaError::ReadFrameError(_) => 9,
            NokhwaError::ProcessFrameError { .. } => 10,
            NokhwaError::StreamShutdownError(_) => 11,
            NokhwaError::UnsupportedOperationError(_) => 12,
            NokhwaError::NotImplementedError(_) => 13,
        }),
        nokhwa,
    )
}

impl Webcam {
    async fn stop(&self, want_image: bool) -> ASCOMResult {
        // Make sure `self.exposing.write()` lock is not held when waiting for `done`.
        let done = match &mut *self.exposing.write() {
            ExposingState::Exposing { stop_tx, done, .. } => {
                // Only send the stop signal if nobody else has.
                if let Some(stop_tx) = stop_tx.take() {
                    _ = stop_tx.send(StopExposure { want_image });
                }
                done.clone()
            }
            ExposingState::Idle { .. } => return Ok(()),
        };
        if done.await {
            Ok(())
        } else {
            Err(ERR_EXPOSURE_FAILED_TO_STOP)
        }
    }
}

#[async_trait]
impl Device for Webcam {
    fn static_name(&self) -> &str {
        &self.name
    }

    fn unique_id(&self) -> &str {
        &self.unique_id
    }

    async fn connected(&self) -> ASCOMResult<bool> {
        match &*self.exposing.read() {
            ExposingState::Idle { camera, .. } => Ok(camera.lock().is_stream_open()),
            ExposingState::Exposing { .. } => Ok(true),
        }
    }

    async fn set_connected(&self, connected: bool) -> ASCOMResult {
        match &*self.exposing.read() {
            ExposingState::Idle { camera, .. } => {
                let mut camera_lock = camera.lock();

                if connected == camera_lock.is_stream_open() {
                    return Ok(());
                }

                if connected {
                    camera_lock.open_stream()
                } else {
                    camera_lock.stop_stream()
                }
                .map_err(convert_err)
            }
            ExposingState::Exposing { .. } => Err(ASCOMError::invalid_operation(
                "Cannot change connection state during an exposure",
            )),
        }
    }

    async fn description(&self) -> ASCOMResult<String> {
        Ok(self.description.clone())
    }

    async fn driver_info(&self) -> ASCOMResult<String> {
        Ok("ascom-alpaca Rust webcam demo".to_owned())
    }

    async fn driver_version(&self) -> ASCOMResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }
}

#[async_trait]
impl AlpacaCamera for Webcam {
    async fn bayer_offset_x(&self) -> ASCOMResult<i32> {
        Ok(0)
    }

    async fn bayer_offset_y(&self) -> ASCOMResult<i32> {
        Ok(0)
    }

    async fn pixel_size_x(&self) -> ASCOMResult<f64> {
        Ok(1.0)
    }

    async fn pixel_size_y(&self) -> ASCOMResult<f64> {
        Ok(1.0)
    }

    async fn sensor_name(&self) -> ASCOMResult<String> {
        Ok(String::default())
    }

    async fn bin_x(&self) -> ASCOMResult<i32> {
        Ok(self.subframe.read().bin.x)
    }

    async fn set_bin_x(&self, bin_x: i32) -> ASCOMResult {
        if self.valid_bins.contains(&bin_x) {
            self.subframe.write().bin.x = bin_x;
            Ok(())
        } else {
            Err(ASCOMError::INVALID_VALUE)
        }
    }

    async fn bin_y(&self) -> ASCOMResult<i32> {
        Ok(self.subframe.read().bin.x)
    }

    async fn set_bin_y(&self, bin_y: i32) -> ASCOMResult {
        if self.valid_bins.contains(&bin_y) {
            self.subframe.write().bin.y = bin_y;
            Ok(())
        } else {
            Err(ASCOMError::INVALID_VALUE)
        }
    }

    async fn max_bin_x(&self) -> ASCOMResult<i32> {
        Ok(*self.valid_bins.last().unwrap())
    }

    async fn max_bin_y(&self) -> ASCOMResult<i32> {
        Ok(*self.valid_bins.last().unwrap())
    }

    async fn camera_state(&self) -> ASCOMResult<CameraState> {
        Ok(match *self.exposing.read() {
            ExposingState::Idle { .. } => CameraState::Idle,
            ExposingState::Exposing { .. } => CameraState::Exposing,
        })
    }

    async fn electrons_per_adu(&self) -> ASCOMResult<f64> {
        Ok(1.)
    }

    async fn exposure_max(&self) -> ASCOMResult<Duration> {
        Ok(Duration::from_secs(10))
    }

    async fn exposure_min(&self) -> ASCOMResult<Duration> {
        self.exposure_resolution().await
    }

    async fn exposure_resolution(&self) -> ASCOMResult<Duration> {
        Ok(Duration::from_secs_f64(
            1. / f64::from(self.max_format.frame_rate()),
        ))
    }

    async fn full_well_capacity(&self) -> ASCOMResult<f64> {
        self.max_adu().await.map(f64::from)
    }

    async fn has_shutter(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    async fn image_array(&self) -> ASCOMResult<ImageArray> {
        match &*self.exposing.read() {
            ExposingState::Idle {
                image: Some(image), ..
            } => Ok(image.clone()),
            _ => Err(ASCOMError::INVALID_OPERATION),
        }
    }

    async fn image_ready(&self) -> ASCOMResult<bool> {
        Ok(matches!(
            *self.exposing.read(),
            ExposingState::Idle { image: Some(_), .. }
        ))
    }

    async fn last_exposure_start_time(&self) -> ASCOMResult<SystemTime> {
        self.last_exposure_start_time
            .read()
            .ok_or(ASCOMError::INVALID_OPERATION)
    }

    async fn last_exposure_duration(&self) -> ASCOMResult<Duration> {
        self.last_exposure_duration
            .read()
            .ok_or(ASCOMError::INVALID_OPERATION)
    }

    async fn max_adu(&self) -> ASCOMResult<i32> {
        Ok(u16::MAX.into())
    }

    async fn camera_x_size(&self) -> ASCOMResult<i32> {
        Ok(self.max_format.width() as i32)
    }

    async fn camera_y_size(&self) -> ASCOMResult<i32> {
        Ok(self.max_format.height() as i32)
    }

    async fn start_x(&self) -> ASCOMResult<i32> {
        Ok(self.subframe.read().offset.x)
    }

    async fn set_start_x(&self, start_x: i32) -> ASCOMResult {
        self.subframe.write().offset.x = start_x;
        Ok(())
    }

    async fn start_y(&self) -> ASCOMResult<i32> {
        Ok(self.subframe.read().offset.y)
    }

    async fn set_start_y(&self, start_y: i32) -> ASCOMResult {
        self.subframe.write().offset.y = start_y;
        Ok(())
    }

    async fn num_x(&self) -> ASCOMResult<i32> {
        Ok(self.subframe.read().size.x)
    }

    async fn set_num_x(&self, num_x: i32) -> ASCOMResult {
        self.subframe.write().size.x = num_x;
        Ok(())
    }

    async fn num_y(&self) -> ASCOMResult<i32> {
        Ok(self.subframe.read().size.y)
    }

    async fn set_num_y(&self, num_y: i32) -> ASCOMResult {
        self.subframe.write().size.y = num_y;
        Ok(())
    }

    #[allow(clippy::cast_possible_truncation)]
    async fn percent_completed(&self) -> ASCOMResult<i32> {
        match &*self.exposing.read() {
            ExposingState::Idle { .. } => Ok(100),
            ExposingState::Exposing {
                start,
                expected_duration,
                ..
            } => Ok((start.elapsed().div_duration_f64(*expected_duration) * 100.) as i32),
        }
    }

    async fn readout_mode(&self) -> ASCOMResult<i32> {
        Ok(0)
    }

    async fn set_readout_mode(&self, readout_mode: i32) -> ASCOMResult {
        if readout_mode == 0 {
            Ok(())
        } else {
            Err(ASCOMError::INVALID_VALUE)
        }
    }

    async fn readout_modes(&self) -> ASCOMResult<Vec<String>> {
        Ok(vec!["Default".to_owned()])
    }

    async fn sensor_type(&self) -> ASCOMResult<SensorType> {
        Ok(SensorType::Color)
    }

    async fn start_exposure(&self, duration: Duration, _light: bool) -> ASCOMResult {
        let exposing_state = Arc::clone(&self.exposing);
        let mut exposing_state_lock = exposing_state.write_arc();
        let camera = match &*exposing_state_lock {
            ExposingState::Idle { camera, .. } => Arc::clone(camera),
            ExposingState::Exposing { .. } => {
                return Err(ASCOMError::invalid_operation("Camera is already exposing"));
            }
        };
        let subframe = self.subframe.read().clone();
        let subframe_end_offset = subframe.offset + subframe.size;
        if subframe.bin.x != subframe.bin.y {
            return Err(ASCOMError::invalid_value("BinX and BinY must be symmetric"));
        }
        let mut camera_lock = camera.lock_arc();
        let mut resolution = self.max_format.resolution();
        resolution.width_x /= subframe.bin.x as u32;
        resolution.height_y /= subframe.bin.y as u32;
        let size = Size::from(resolution);
        if subframe.offset < Point::default()
            || subframe.offset + subframe.size > Point::default() + size
        {
            return Err(ASCOMError::invalid_value("Subframe is out of bounds"));
        }
        let mut format = self.max_format;
        if camera_lock.resolution() != resolution {
            // Recreate camera completely because `set_camera_requset` is currently buggy.
            // See https://github.com/l1npengtul/nokhwa/issues/111.
            let index = camera_lock.index().clone();
            format.set_resolution(resolution);
            *camera_lock = Camera::new(
                index,
                RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(format)),
            )
            .map_err(|err| {
                tracing::error!("Couldn't change camera format: {err}");
                convert_err(err)
            })?;
            camera_lock.open_stream().map_err(convert_err)?;
        }
        let last_exposure_duration = Arc::clone(&self.last_exposure_duration);
        let (stop_tx, stop_rx) = oneshot::channel::<StopExposure>();
        // Run long blocking exposing operation on a dedicated I/O thread.
        let (frames_tx, mut frames_rx) =
            mpsc::unbounded_channel::<Result<nokhwa::Buffer, NokhwaError>>();
        *self.last_exposure_start_time.write() = Some(SystemTime::now());
        let start = Instant::now();

        let frame_reader_task = task::spawn_blocking(move || {
            // Webcams produce variable-length exposures, so we can't just precalculate count of
            // frames and instead need to check the total elapsed time.
            loop {
                let frame_res = camera_lock.frame();
                let failed = frame_res.is_err();
                let total_duration = start.elapsed();
                if frames_tx.send(frame_res).is_err() || failed || total_duration >= duration {
                    // Receiver was dropped due to stop_exposure or abort_exposure or retrieving frame failed.
                    // Either way, stop exposing.
                    return total_duration;
                }
            }
        });

        let task = task::spawn(async move {
            let mut stacked_buffer =
                Array3::<u16>::zeros((subframe.size.y as usize, subframe.size.x as usize, 3));
            // Watches `stop` channel and the actual exposure for whichever ends the exposure first.
            let stop_res = tokio::select! {
                stop_res = stop_rx => stop_res.map_err(|_err| ERR_EXPOSING_STATE_CHANGED_UNEXPECTEDLY),
                stop_res = async {
                    let mut single_frame_buffer = Array3::<u8>::zeros((size.y as usize, size.x as usize, 3));
                    while let Some(frame_res) = frames_rx.recv().await {
                        let frame = frame_res.map_err(convert_err)?;

                        frame.decode_image_to_buffer::<RgbFormat>(single_frame_buffer.as_slice_mut().unwrap()).map_err(convert_err)?;

                        let cropped_view = single_frame_buffer.slice(ndarray::s![
                            subframe.offset.y..subframe_end_offset.y,
                            subframe.offset.x..subframe_end_offset.x,
                            ..
                        ]);

                        ndarray::par_azip!((&src in cropped_view, dst in &mut stacked_buffer) {
                            *dst = dst.saturating_add(src.into());
                        });
                    }
                    Ok(StopExposure { want_image: true })
                } => stop_res,
            };
            let stop = match stop_res {
                Ok(stop) => stop,
                Err(err) => {
                    tracing::error!(%err, "Exposure stopped prematurely due to an error");
                    StopExposure { want_image: false }
                }
            };
            *exposing_state.write() = ExposingState::Idle {
                camera,
                image: stop.want_image.then(|| {
                    // Swap axes from image representation (y then x) to array representation (x then y).
                    stacked_buffer.swap_axes(0, 1);
                    stacked_buffer.into()
                }),
            };
            *last_exposure_duration.write() = Some(frame_reader_task.await.unwrap());
        });

        *exposing_state_lock = ExposingState::Exposing {
            start,
            expected_duration: duration,
            stop_tx: Some(stop_tx),
            done: task.map(|res| res.is_ok()).boxed().shared(),
        };

        Ok(())
    }

    async fn can_stop_exposure(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    async fn can_abort_exposure(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    async fn stop_exposure(&self) -> ASCOMResult {
        self.stop(true).await
    }

    async fn abort_exposure(&self) -> ASCOMResult {
        self.stop(false).await
    }
}

fn exact_div(a: u32, b: u32) -> Option<u32> {
    a.is_multiple_of(b).then_some(a / b)
}

#[tracing::instrument(ret(level = "debug"), err)]
fn get_webcam(camera_info: &CameraInfo) -> eyre::Result<Webcam> {
    // Workaround for https://github.com/l1npengtul/nokhwa/issues/110:
    // get list of compatible formats manually, extract the info,
    // and then re-create as Camera for the same source.
    let mut camera = Camera::new(
        camera_info.index().clone(),
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::None),
    )?;

    let compatible_formats = camera.compatible_camera_formats()?;

    let format = *compatible_formats
        .iter()
        .max_by_key(|format| {
            (
                // nokhwa's auto-selection doesn't care about specific frame format,
                // but we want uncompressed RAWRGB if supported.
                format.format() == FrameFormat::RAWRGB,
                // then choose maximum resolution
                format.resolution(),
                // then choose *minimum* frame rate as our usecase is mostly long exposures
                -(format.frame_rate() as i32),
            )
        })
        .with_context(|| {
            format!(
                "No compatible formats found for camera {}",
                camera_info.human_name()
            )
        })?;

    let mut valid_bins = compatible_formats
        .iter()
        .filter(|other| {
            format.format() == other.format() && format.frame_rate() == other.frame_rate()
        })
        .filter_map(|other| {
            let bin_x = exact_div(format.resolution().x(), other.resolution().x())?;
            let bin_y = exact_div(format.resolution().y(), other.resolution().y())?;

            (bin_x == bin_y).then_some(bin_x as i32)
        })
        .collect::<Vec<_>>();

    valid_bins.sort_unstable();
    valid_bins.dedup();

    let camera = Camera::new(
        camera_info.index().clone(),
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(format)),
    )?;

    Ok(Webcam {
        unique_id: format!(
            "150ddacb-7ad9-4754-b289-ae56210693e8::{}",
            camera_info.index()
        ),
        name: camera_info.human_name(),
        description: camera_info.description().to_owned(),
        subframe: RwLock::new(Subframe {
            offset: Point::default(),
            size: format.resolution().into(),
            bin: Size { x: 1, y: 1 },
        }),
        max_format: format,
        valid_bins,
        exposing: Arc::new(RwLock::new(ExposingState::Idle {
            camera: Arc::new(Mutex::new(camera)),
            image: None,
        })),
        last_exposure_start_time: RwLock::default(),
        last_exposure_duration: Arc::default(),
    })
}

#[tokio::main]
async fn main() -> eyre::Result<std::convert::Infallible> {
    tracing_subscriber::fmt::init();
    setup_server().await?.start().await
}

async fn setup_server() -> eyre::Result<Server> {
    {
        let (init_tx, init_rx) = oneshot::channel();
        // Ideally this would be *just* oneshot but can't be due to https://github.com/l1npengtul/nokhwa/issues/109.
        let init_tx = Mutex::new(Some(init_tx));
        nokhwa_initialize(move |status| {
            _ = init_tx
                .lock()
                .take()
                .expect("this is semantically oneshot and must never fail")
                .send(status);
        });
        eyre::ensure!(init_rx.await?, "User did not grant camera access");
    }

    let mut server = Server::new(CargoServerInfo!());
    server.listen_addr = (Ipv4Addr::LOCALHOST, 8000).into();

    for camera_info in nokhwa::query(nokhwa::utils::ApiBackend::Auto)? {
        if let Ok(webcam) = get_webcam(&camera_info) {
            server.devices.register(webcam);
        }
    }

    Ok(server)
}

#[cfg(test)]
#[tokio::test]
async fn run_conformu_tests() -> eyre::Result<()> {
    use ascom_alpaca::test::run_conformu_tests;
    use futures::future::try_join_all;

    tracing_subscriber::fmt::init();

    let server = setup_server().await?;

    let server_url = format!("http://{}/", server.listen_addr);
    let webcam_count = server.devices.iter::<dyn AlpacaCamera>().len();

    tokio::select! {
        proxy_result = server.start() => match proxy_result? {},

        tests_result = try_join_all((0..webcam_count).map(|i| {
            run_conformu_tests::<dyn AlpacaCamera>(&server_url, i)
        })) => tests_result.map(|_: Vec<()>| ()),
    }
}
