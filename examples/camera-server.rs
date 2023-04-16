use ascom_alpaca::api::{Camera, CameraState, CargoServerInfo, Device, ImageArray};
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult, Server};
use async_trait::async_trait;
use image::{GenericImageView, Pixel, RgbImage};
use ndarray::Array3;
use net_literals::addr;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::Resolution;
use nokhwa::utils::{CameraFormat, FrameFormat, RequestedFormat, RequestedFormatType};
use nokhwa::{nokhwa_initialize, Buffer, CallbackCamera, NokhwaError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::OnceCell;

#[derive(Debug, Clone, Copy)]
struct Vec2 {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Config {
    state: CameraState,
    subframe_offset: Vec2,
    subframe_size: Vec2,
}

#[derive(Default)]
struct ExposingState {
    image: OnceCell<ImageArray>,
    stop_exposure: AtomicBool,
}

#[derive(custom_debug::Debug)]
struct Webcam {
    unique_id: String,
    name: String,
    format: CameraFormat,
    config: Arc<RwLock<Config>>,
    frame_sender: tokio::sync::broadcast::Sender<Buffer>,
    #[debug(skip)]
    camera: RwLock<CallbackCamera>,
    #[debug(skip)]
    exposing: Arc<RwLock<ExposingState>>,
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
        self.camera
            .read()
            .unwrap()
            .is_stream_open()
            .map_err(convert_err)
    }

    async fn set_connected(&self, connected: bool) -> ASCOMResult {
        let mut camera = self.camera.write().unwrap();

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
        Ok(self.camera.read().unwrap().info().description().to_owned())
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
        Ok(1)
    }

    async fn set_bin_x(&self, bin_x: i32) -> ASCOMResult {
        if bin_x == 1 {
            Ok(())
        } else {
            Err(ASCOMError::INVALID_VALUE)
        }
    }

    async fn bin_y(&self) -> ASCOMResult<i32> {
        self.bin_x().await
    }

    async fn set_bin_y(&self, bin_y: i32) -> ASCOMResult {
        self.set_bin_x(bin_y).await
    }

    async fn camera_state(&self) -> ASCOMResult<ascom_alpaca::api::CameraState> {
        Ok(self.config.read().unwrap().state)
    }

    async fn camera_xsize(&self) -> ASCOMResult<i32> {
        Ok(self.format.width() as i32)
    }

    async fn camera_ysize(&self) -> ASCOMResult<i32> {
        Ok(self.format.height() as i32)
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
        Ok(1. / f64::from(self.format.frame_rate()))
    }

    async fn full_well_capacity(&self) -> ASCOMResult<f64> {
        self.max_adu().await.map(f64::from)
    }

    async fn has_shutter(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    async fn image_array(&self) -> ASCOMResult<ascom_alpaca::api::ImageArray> {
        self.exposing
            .read()
            .unwrap()
            .image
            .get()
            .cloned()
            .ok_or(ASCOMError::INVALID_OPERATION)
    }

    async fn image_ready(&self) -> ASCOMResult<bool> {
        Ok(self.exposing.read().unwrap().image.initialized())
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

    async fn max_bin_x(&self) -> ASCOMResult<i32> {
        Ok(1)
    }

    async fn max_bin_y(&self) -> ASCOMResult<i32> {
        Ok(1)
    }

    async fn num_x(&self) -> ASCOMResult<i32> {
        Ok(self.config.read().unwrap().subframe_size.x)
    }

    async fn set_num_x(&self, num_x: i32) -> ASCOMResult {
        self.config.write().unwrap().subframe_size.x = num_x;
        Ok(())
    }

    async fn num_y(&self) -> ASCOMResult<i32> {
        Ok(self.config.read().unwrap().subframe_size.y)
    }

    async fn set_num_y(&self, num_y: i32) -> ASCOMResult {
        self.config.write().unwrap().subframe_size.y = num_y;
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

    async fn start_x(&self) -> ASCOMResult<i32> {
        Ok(self.config.read().unwrap().subframe_offset.x)
    }

    async fn set_start_x(&self, start_x: i32) -> ASCOMResult {
        self.config.write().unwrap().subframe_offset.x = start_x;
        Ok(())
    }

    async fn start_y(&self) -> ASCOMResult<i32> {
        Ok(self.config.read().unwrap().subframe_offset.y)
    }

    async fn set_start_y(&self, start_y: i32) -> ASCOMResult {
        self.config.write().unwrap().subframe_offset.y = start_y;
        Ok(())
    }

    async fn start_exposure(&self, duration: f64, _light: bool) -> ASCOMResult {
        let mut config = self.config.write().unwrap();
        if config.state != CameraState::Idle {
            return Err(ASCOMError::INVALID_OPERATION);
        }
        {
            let subframe_offset = config.subframe_offset;
            let subframe_size = config.subframe_size;
            let config = self.config.clone();
            let count = (duration * f64::from(self.format.frame_rate())) as u8;
            let exposing = self.exposing.clone();
            *exposing.write().unwrap() = ExposingState::default();
            let resolution = self.format.resolution();
            let mut frame_receiver = self.frame_sender.subscribe();
            tokio::spawn(async move {
                tracing::info!("Starting exposure");
                let mut stacked_buffer =
                    Array3::<u8>::zeros((resolution.x() as usize, resolution.y() as usize, 3));
                let mut single_frame_buffer = RgbImage::new(resolution.x(), resolution.y());
                for _ in 0..count {
                    if exposing
                        .read()
                        .unwrap()
                        .stop_exposure
                        .load(Ordering::Relaxed)
                    {
                        break;
                    }
                    frame_receiver
                        .recv()
                        .await
                        .unwrap()
                        .decode_image_to_buffer::<RgbFormat>(&mut single_frame_buffer)
                        .unwrap();
                    let cropped_view = single_frame_buffer.view(
                        subframe_offset.x as u32,
                        subframe_offset.y as u32,
                        subframe_size.x as u32,
                        subframe_size.y as u32,
                    );
                    for (x, y, pixel) in cropped_view.pixels() {
                        stacked_buffer
                            .slice_mut(ndarray::s![x as usize, y as usize, ..])
                            .iter_mut()
                            .zip(pixel.channels())
                            .for_each(|(dst, src)| *dst = dst.saturating_add(*src));
                    }
                }
                config.write().unwrap().state = CameraState::Idle;
                exposing
                    .write()
                    .unwrap()
                    .image
                    .set(stacked_buffer.into())
                    .unwrap();
                tracing::info!("Finished exposure");
            });
        }
        config.state = CameraState::Exposing;
        Ok(())
    }

    async fn sensor_type(&self) -> ascom_alpaca::ASCOMResult<ascom_alpaca::api::SensorType> {
        Ok(ascom_alpaca::api::SensorType::Color)
    }

    async fn can_stop_exposure(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    async fn can_abort_exposure(&self) -> ASCOMResult<bool> {
        self.can_stop_exposure().await
    }

    async fn stop_exposure(&self) -> ASCOMResult {
        self.exposing
            .read()
            .unwrap()
            .stop_exposure
            .store(true, Ordering::Relaxed);
        self.config.write().unwrap().state = CameraState::Idle;
        Ok(())
    }

    async fn abort_exposure(&self) -> ASCOMResult {
        self.stop_exposure().await
    }
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
        let (frame_sender, _) = tokio::sync::broadcast::channel(1);

        let mut camera = CallbackCamera::new(
            camera_info.index().clone(),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::None),
            {
                let frame_sender = frame_sender.clone();
                move |frame| {
                    let _ignore_err_when_no_receivers = frame_sender.send(frame);
                }
            },
        )?;

        let frame_rate = camera.frame_rate()?;
        let format = camera.camera_format()?;

        tracing::debug!(?format, ?frame_rate, "Camera format chosen out of");

        let webcam = Webcam {
            unique_id: format!(
                "150ddacb-7ad9-4754-b289-ae56210693e8::{}",
                camera_info.index()
            ),
            name: camera_info.human_name(),
            config: Arc::new(RwLock::new(Config {
                state: CameraState::Idle,
                subframe_offset: Vec2 { x: 0, y: 0 },
                subframe_size: Vec2 {
                    x: format.width() as _,
                    y: format.height() as _,
                },
            })),
            format,
            camera: RwLock::new(camera),
            exposing: Default::default(),
            frame_sender,
        };

        tracing::debug!(?webcam, "Registering webcam");

        server.devices.register(webcam);
    }

    server.start().await
}
