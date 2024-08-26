#[cfg(feature = "client")]
mod client;
#[cfg(feature = "server")]
mod server;

#[cfg(feature = "server")]
pub(crate) use server::ImageBytesResponse;

use bytemuck::{AnyBitPattern, Pod, Zeroable};
use ndarray::{Array2, Array3, ArrayView2, ArrayView3, Axis};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::num::NonZeroU32;
use std::ops::Deref;

// Missing alias in ndarray.
type ArcArray3<T> = ndarray::ArcArray<T, ndarray::Ix3>;

/// Rank of an image array.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Serialize_repr,
    Deserialize_repr,
    TryFromPrimitive,
    IntoPrimitive,
)]
#[repr(i32)]
pub enum ImageArrayRank {
    /// 2D
    Rank2 = 2_i32,
    /// 3D
    Rank3 = 3_i32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub(crate) enum TransmissionElementType {
    I16 = 1,
    I32 = 2,
    U8 = 6,
    U16 = 8,
}

// Limited to the only supported element type; useful for serde purposes.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(i32)]
pub(crate) enum ImageElementType {
    I32 = TransmissionElementType::I32 as i32,
}

trait AsTransmissionElementType: 'static + Into<i32> + AnyBitPattern {
    const TYPE: TransmissionElementType;
}

impl AsTransmissionElementType for i16 {
    const TYPE: TransmissionElementType = TransmissionElementType::I16;
}

impl AsTransmissionElementType for i32 {
    const TYPE: TransmissionElementType = TransmissionElementType::I32;
}

impl AsTransmissionElementType for u16 {
    const TYPE: TransmissionElementType = TransmissionElementType::U16;
}

impl AsTransmissionElementType for u8 {
    const TYPE: TransmissionElementType = TransmissionElementType::U8;
}

/// Image array.
///
/// Image is represented as a 3D array regardless of its actual rank.
/// If the image is a 2D image, the third dimension will have length 1.
///
/// You can retrieve rank as an enum via the [`ImageArray::rank`] method.
///
/// This type is cheaply clonable.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ImageArray {
    data: ArcArray3<i32>,
    transmission_element_type: TransmissionElementType,
}

const COLOUR_AXIS: Axis = Axis(2);

impl<T: AsTransmissionElementType> From<ArrayView3<'_, T>> for ImageArray {
    fn from(array: ArrayView3<'_, T>) -> Self {
        let data = array.mapv(Into::into);
        let transmission_element_type = T::TYPE;
        Self {
            data: data.into_shared(),
            transmission_element_type,
        }
    }
}

impl<T: AsTransmissionElementType> From<Array3<T>> for ImageArray {
    fn from(array: Array3<T>) -> Self {
        let data = array.mapv_into_any(Into::into);
        let transmission_element_type = T::TYPE;
        Self {
            data: data.into_shared(),
            transmission_element_type,
        }
    }
}

impl<T: AsTransmissionElementType> From<ArrayView2<'_, T>> for ImageArray {
    fn from(array: ArrayView2<'_, T>) -> Self {
        array.insert_axis(COLOUR_AXIS).into()
    }
}

impl<T: AsTransmissionElementType> From<Array2<T>> for ImageArray {
    fn from(array: Array2<T>) -> Self {
        array.insert_axis(COLOUR_AXIS).into()
    }
}

impl Deref for ImageArray {
    type Target = ArcArray3<i32>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

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
