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
    /// See [`TransmissionElementType::I32`].
    I32 = 2,
}

trait AsTransmissionElementType: 'static + Into<i32> + AnyBitPattern {
    const TYPE: TransmissionElementType;
}

/// Decode one little-endian element from a fixed-size `&[u8; N]` chunk
/// and widen it to `i32`. The const `N` ties the byte-chunk width to
/// the element type at the type level (`i16` / `u16` => 2, `i32` => 4,
/// `u8` => 1), so a chunk width that disagrees with the element is a
/// compile error and the per-pixel conversions carry no bounds checks.
/// Used by the unaligned-input slow path of `cast_raw_data` (see
/// `image_array/client.rs`); the aligned fast path still reinterprets
/// in place via `bytemuck::try_cast_slice`.
#[cfg(feature = "client")]
trait WidenLeChunk<const N: usize> {
    fn widen_from_le_chunk(chunk: &[u8; N]) -> i32;
}

impl AsTransmissionElementType for i16 {
    const TYPE: TransmissionElementType = TransmissionElementType::I16;
}

#[cfg(feature = "client")]
impl WidenLeChunk<2> for i16 {
    fn widen_from_le_chunk(chunk: &[u8; 2]) -> i32 {
        i32::from(Self::from_le_bytes(*chunk))
    }
}

impl AsTransmissionElementType for i32 {
    const TYPE: TransmissionElementType = TransmissionElementType::I32;
}

#[cfg(feature = "client")]
impl WidenLeChunk<4> for i32 {
    fn widen_from_le_chunk(chunk: &[u8; 4]) -> i32 {
        Self::from_le_bytes(*chunk)
    }
}

impl AsTransmissionElementType for u16 {
    const TYPE: TransmissionElementType = TransmissionElementType::U16;
}

#[cfg(feature = "client")]
impl WidenLeChunk<2> for u16 {
    fn widen_from_le_chunk(chunk: &[u8; 2]) -> i32 {
        i32::from(Self::from_le_bytes(*chunk))
    }
}

impl AsTransmissionElementType for u8 {
    const TYPE: TransmissionElementType = TransmissionElementType::U8;
}

#[cfg(feature = "client")]
impl WidenLeChunk<1> for u8 {
    fn widen_from_le_chunk(chunk: &[u8; 1]) -> i32 {
        i32::from(Self::from_le_bytes(*chunk))
    }
}

/// Image array.
///
/// Image is represented as a 3D array regardless of its actual rank.
/// If the image is a 2D image, the third dimension will have length 1.
///
/// You can retrieve rank as an enum via the [`ImageArray::rank`] method.
///
/// This type is cheaply clonable.
#[derive(Debug, PartialEq, Eq, Clone, derive_more::Deref)]
pub struct ImageArray {
    #[deref]
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
