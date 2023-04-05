#[cfg(feature = "client")]
mod client;
#[cfg(feature = "server")]
mod server;

#[cfg(feature = "server")]
pub(crate) use server::ImageBytesResponse;

use bytemuck::{Pod, Zeroable};
use ndarray::{Array3, Axis};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::num::NonZeroU32;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ImageArrayResponseRank {
    Rank2 = 2,
    Rank3 = 3,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ImageArrayResponse {
    pub data: Array3<i32>,
}

const COLOUR_AXIS: Axis = Axis(2);

impl ImageArrayResponse {
    pub fn rank(&self) -> ImageArrayResponseRank {
        match self.data.len_of(COLOUR_AXIS) {
            1 => ImageArrayResponseRank::Rank2,
            _ => ImageArrayResponseRank::Rank3,
        }
    }
}

#[cfg(not(target_endian = "little"))]
compile_error!(
"Image handling is currently only supported on little-endian platforms for simplicity & performance.
If you have a real-world use case for big-endian support, please open an issue on GitHub."
);

#[cfg(any(feature = "client", feature = "server"))]
#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct ImageBytesMetadata {
    metadata_version: i32,
    error_number: i32,
    client_transaction_id: Option<NonZeroU32>,
    server_transaction_id: Option<NonZeroU32>,
    data_start: i32,
    image_element_type: i32,
    transmission_element_type: i32,
    rank: i32,
    dimension_1: i32,
    dimension_2: i32,
    dimension_3: i32,
}

const IMAGE_BYTES_TYPE: &str = "application/imagebytes";
