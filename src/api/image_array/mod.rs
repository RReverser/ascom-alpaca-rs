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

/// Rank of an image array.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ImageArrayRank {
    /// 2D
    Rank2 = 2,
    /// 3D
    Rank3 = 3,
}

/// Image array.
///
/// Image is represented as a 3D array regardless of its actual rank.
/// If the image is a 2D image, the third dimension will have length 1.
///
/// You can retrieve rank as an enum via the [`ImageArray::rank`] method.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ImageArray {
    /// Image data.
    pub data: Array3<i32>,
}

const COLOUR_AXIS: Axis = Axis(2);

impl ImageArray {
    /// Retrieve actual rank of the image.
    pub fn rank(&self) -> ImageArrayRank {
        match self.data.len_of(COLOUR_AXIS) {
            1 => ImageArrayRank::Rank2,
            _ => ImageArrayRank::Rank3,
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
