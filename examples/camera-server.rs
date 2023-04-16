use anyhow::Context;
use ascom_alpaca::api::{Camera, CameraState, CargoServerInfo, Device, ImageArray, SensorType};
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult, Server};
use async_trait::async_trait;
use image::{GenericImageView, Pixel, RgbImage};
use ndarray::Array3;
use net_literals::addr;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraFormat, FrameFormat, RequestedFormat, RequestedFormatType};
use nokhwa::{nokhwa_initialize, Buffer, CallbackCamera, NokhwaError};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
struct Vec2 {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy)]
struct Subframe {
    offset: Vec2,
    size: Vec2,
}

enum ExposureStopKind {
    Normal,
    Stop,
    Abort,
}

enum ExposingState {
    BeforeExposure,
    Exposing {
        id: usize,
        frame_tx: tokio::sync::mpsc::Sender<Buffer>,
        stop_exposure_tx: tokio::sync::watch::Sender<ExposureStopKind>,
    },
    AfterExposure {
        image: ImageArray,
    },
}

#[derive(Debug, Clone, Copy)]
struct BinnedFormat {
    bin: u32,
    format: CameraFormat,
}

#[derive(custom_debug::Debug)]
struct Webcam {
    unique_id: String,
    name: String,
    binned_format: RwLock<BinnedFormat>,
    subframe: RwLock<Subframe>,
    #[debug(skip)]
    camera: RwLock<CallbackCamera>,
    #[debug(skip)]
    exposing: Arc<RwLock<ExposingState>>,
    exposure_counter: AtomicUsize,
    binning_formats: Vec<BinnedFormat>,
}

fn convert_err(nokhwa: NokhwaError) -> ASCOMError {
    // TODO: more granular errors
    ASCOMError::new(ASCOMErrorCode::UNSPECIFIED, nokhwa.to_string())
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
        self.camera.read().is_stream_open().map_err(convert_err)
    }

    async fn set_connected(&self, connected: bool) -> ASCOMResult {
        let mut camera = self.camera.write();

        if connected == camera.is_stream_open().map_err(convert_err)? {
            return Ok(());
        }

        if connected {
            camera.open_stream()
        } else {
            camera.stop_stream()
        }
        .map_err(convert_err)
    }

    async fn description(&self) -> ASCOMResult<String> {
        Ok(self.camera.read().info().description().to_owned())
    }

    async fn driver_info(&self) -> ASCOMResult<String> {
        Ok("ascom-alpaca Rust webcam demo".to_owned())
    }

    async fn driver_version(&self) -> ASCOMResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }

    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(3)
    }

    async fn name(&self) -> ASCOMResult<String> {
        Ok(self.name.clone())
    }

    async fn supported_actions(&self) -> ASCOMResult<Vec<String>> {
        Ok(vec![])
    }
}

#[async_trait]
impl Camera for Webcam {
    async fn bayer_offset_x(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn bayer_offset_y(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn bin_x(&self) -> ASCOMResult<i32> {
        Ok(self.binned_format.read().bin as i32)
    }

    async fn set_bin_x(&self, bin_x: i32) -> ASCOMResult {
        self.binning_formats
            .iter()
            .find(|binned_format| binned_format.bin == bin_x as u32)
            .ok_or(ASCOMError::INVALID_VALUE)
            .map(|binned_format| {
                *self.binned_format.write() = *binned_format;
            })
    }

    async fn bin_y(&self) -> ASCOMResult<i32> {
        self.bin_x().await
    }

    async fn set_bin_y(&self, bin_y: i32) -> ASCOMResult {
        self.set_bin_x(bin_y).await
    }

    async fn max_bin_x(&self) -> ASCOMResult<i32> {
        Ok(self.binning_formats.last().unwrap().bin as i32)
    }

    async fn max_bin_y(&self) -> ASCOMResult<i32> {
        Ok(self.binning_formats.last().unwrap().bin as i32)
    }

    async fn camera_state(&self) -> ASCOMResult<CameraState> {
        Ok(match *self.exposing.read() {
            ExposingState::BeforeExposure | ExposingState::AfterExposure { .. } => {
                CameraState::Idle
            }
            ExposingState::Exposing { .. } => CameraState::Exposing,
        })
    }

    async fn can_asymmetric_bin(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    async fn can_fast_readout(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    async fn can_get_cooler_power(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    async fn can_pulse_guide(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    async fn can_set_ccd_temperature(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    async fn electrons_per_adu(&self) -> ASCOMResult<f64> {
        Ok(1.)
    }

    async fn exposure_max(&self) -> ASCOMResult<f64> {
        Ok(self.exposure_resolution().await? * f64::from(u8::MAX))
    }

    async fn exposure_min(&self) -> ASCOMResult<f64> {
        self.exposure_resolution().await
    }

    async fn exposure_resolution(&self) -> ASCOMResult<f64> {
        Ok(1. / f64::from(self.binned_format.read().format.frame_rate()))
    }

    async fn full_well_capacity(&self) -> ASCOMResult<f64> {
        self.max_adu().await.map(f64::from)
    }

    async fn has_shutter(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    async fn image_array(&self) -> ASCOMResult<ImageArray> {
        match &*self.exposing.read() {
            ExposingState::AfterExposure { image } => Ok(image.clone()),
            _ => Err(ASCOMError::INVALID_OPERATION),
        }
    }

    async fn image_ready(&self) -> ASCOMResult<bool> {
        Ok(matches!(
            *self.exposing.read(),
            ExposingState::AfterExposure { .. }
        ))
    }

    async fn last_exposure_duration(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn last_exposure_start_time(&self) -> ASCOMResult<String> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn max_adu(&self) -> ASCOMResult<i32> {
        Ok(u16::MAX.into())
    }

    async fn camera_xsize(&self) -> ASCOMResult<i32> {
        Ok(self.binned_format.read().format.width() as i32)
    }

    async fn camera_ysize(&self) -> ASCOMResult<i32> {
        Ok(self.binned_format.read().format.height() as i32)
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

    async fn percent_completed(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
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
        Ok(vec!["Default".to_string()])
    }

    async fn sensor_type(&self) -> ascom_alpaca::ASCOMResult<SensorType> {
        Ok(SensorType::Color)
    }

    async fn start_exposure(&self, duration: f64, _light: bool) -> ASCOMResult {
        let exposing_state = self.exposing.clone();
        let mut exposing_state_lock = self.exposing.write();
        if let ExposingState::Exposing {
            stop_exposure_tx, ..
        } = &*exposing_state_lock
        {
            // It's possible that the state is still Exposing, but something has already
            // sent a signal to abort exposure. If so, it's okay to start a new exposure.
            if let ExposureStopKind::Normal = &*stop_exposure_tx.borrow() {
                return Err(ASCOMError::INVALID_OPERATION);
            }
        }
        let format = self.binned_format.read().format;
        let count = (duration * f64::from(format.frame_rate())).round() as usize;
        let (frame_tx, mut frame_rx) = tokio::sync::mpsc::channel(count);
        let (stop_exposure_tx, mut stop_exposure_rx) =
            tokio::sync::watch::channel(ExposureStopKind::Normal);
        let id = self.exposure_counter.fetch_add(1, Ordering::Relaxed);
        *exposing_state_lock = ExposingState::Exposing {
            id,
            frame_tx,
            stop_exposure_tx,
        };
        drop(exposing_state_lock);
        let subframe = *self.subframe.read();
        let resolution = format.resolution();
        tokio::spawn(async move {
            let mut stacked_buffer =
                Array3::<u16>::zeros((resolution.x() as usize, resolution.y() as usize, 3));
            let mut single_frame_buffer = RgbImage::new(resolution.x(), resolution.y());
            tokio::select! {
                _ = stop_exposure_rx.changed() => {}
                _ = async {
                    for _ in 0..count {
                        let Some(frame) = frame_rx
                            .recv()
                            .await else {
                                break;
                            };
                        frame
                            .decode_image_to_buffer::<RgbFormat>(&mut single_frame_buffer)
                            .unwrap();
                        let cropped_view = single_frame_buffer.view(
                            subframe.offset.x as u32,
                            subframe.offset.y as u32,
                            subframe.size.x as u32,
                            subframe.size.y as u32,
                        );
                        for (x, y, pixel) in cropped_view.pixels() {
                            stacked_buffer
                                .slice_mut(ndarray::s![x as usize, y as usize, ..])
                                .iter_mut()
                                .zip(pixel.channels())
                                .for_each(|(dst, src)| *dst = dst.saturating_add((*src).into()));
                        }
                    }
                } => {}
            }
            let mut exposing_state_lock = exposing_state.write();
            if let ExposingState::Exposing { id: current_id, .. } = &mut *exposing_state_lock {
                // Check that another exposure hasn't started in between
                if *current_id == id {
                    *exposing_state_lock = match &*stop_exposure_rx.borrow() {
                        ExposureStopKind::Normal | ExposureStopKind::Stop => {
                            ExposingState::AfterExposure {
                                image: stacked_buffer.into(),
                            }
                        }
                        ExposureStopKind::Abort => ExposingState::BeforeExposure,
                    };
                }
            }
        });
        Ok(())
    }

    async fn can_stop_exposure(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    async fn can_abort_exposure(&self) -> ASCOMResult<bool> {
        self.can_stop_exposure().await
    }

    async fn stop_exposure(&self) -> ASCOMResult {
        if let ExposingState::Exposing {
            stop_exposure_tx, ..
        } = &*self.exposing.read()
        {
            let _ = stop_exposure_tx.send(ExposureStopKind::Stop);
        }
        Ok(())
    }

    async fn abort_exposure(&self) -> ASCOMResult {
        if let ExposingState::Exposing {
            stop_exposure_tx, ..
        } = &*self.exposing.read()
        {
            let _ = stop_exposure_tx.send(ExposureStopKind::Abort);
        }
        Ok(())
    }
}

fn div_rem(a: u32, b: u32) -> (u32, u32) {
    (a / b, a % b)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let (init_tx, init_rx) = std::sync::mpsc::sync_channel(1);
    nokhwa_initialize(move |status| {
        init_tx.send(status).unwrap();
    });
    anyhow::ensure!(init_rx.recv()?, "User did not grant camera access");

    let mut server = Server {
        info: CargoServerInfo!(),
        listen_addr: addr!("0.0.0.0:8000"),
        ..Default::default()
    };
    for camera_info in nokhwa::query(nokhwa::utils::ApiBackend::Auto)? {
        // Workaround for https://github.com/l1npengtul/nokhwa/issues/110:
        // get list of compatible formats manually, extract the info,
        // and then re-create as CallbackCamera for the same source.
        let mut camera = nokhwa::Camera::new(
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

        let mut binning_formats = compatible_formats
            .iter()
            .filter_map(|other| {
                let (bin_x, rem_x) = div_rem(format.resolution().x(), other.resolution().x());
                let (bin_y, rem_y) = div_rem(format.resolution().y(), other.resolution().y());
                if bin_x == bin_y && rem_x == 0 && rem_y == 0 {
                    Some(BinnedFormat {
                        bin: bin_x,
                        format: *other,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        binning_formats.sort_by_key(|binned_format| binned_format.bin);

        let exposing = Arc::new(RwLock::new(ExposingState::BeforeExposure));

        let camera = CallbackCamera::new(
            camera_info.index().clone(),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(format)),
            {
                let exposing = exposing.clone();
                move |frame| {
                    if let ExposingState::Exposing { frame_tx, .. } = &*exposing.read() {
                        frame_tx.blocking_send(frame).unwrap();
                    }
                }
            },
        )?;

        let webcam = Webcam {
            unique_id: format!(
                "150ddacb-7ad9-4754-b289-ae56210693e8::{}",
                camera_info.index()
            ),
            name: camera_info.human_name(),
            subframe: RwLock::new(Subframe {
                offset: Vec2 { x: 0, y: 0 },
                size: Vec2 {
                    x: format.width() as _,
                    y: format.height() as _,
                },
            }),
            binned_format: RwLock::new(BinnedFormat { bin: 1, format }),
            binning_formats,
            camera: RwLock::new(camera),
            exposing,
            exposure_counter: AtomicUsize::new(0),
        };

        tracing::debug!(?webcam, "Registering webcam");

        server.devices.register(webcam);
    }

    server.start().await
}
