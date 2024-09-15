use super::Response;
use crate::api::{ConfiguredDevice, FallibleDeviceType, ImageArray};
use crate::response::ValueResponse;
use crate::ASCOMResult;
use bytes::Bytes;
use criterion::Criterion;
use mime::APPLICATION_JSON;

macro_rules! declare_parsing_benches {
    ($($name:ident: $ty:ty => ($mime:expr, $fixture_path:literal),)*) => {
        /// Run response parsing benchmarks against stored fixtures.
        pub fn benches() {
            let _ =
                Criterion::default()
                .configure_from_args()
                $(
                    .bench_function(stringify!($name), |b| {
                        b.iter(move || {
                            <$ty>::from_reqwest(
                                $mime,
                                Bytes::from_static(include_bytes!($fixture_path)),
                            )
                            .expect("Failed to parse fixture")
                        });
                    })
                )*;
        }
    };
}

declare_parsing_benches! {
    parse_configured_devices: ValueResponse<Vec<ConfiguredDevice<FallibleDeviceType>>>
        => (APPLICATION_JSON, "resp_configured_devices.json"),
    parse_image_array: ASCOMResult<ImageArray>
        => (APPLICATION_JSON, "resp_image_array.json"),
    parse_image_array_bin: ASCOMResult<ImageArray>
        => ("application/imagebytes".parse().expect("couldn't parse mime type"), "resp_image_array.bin"),
}
