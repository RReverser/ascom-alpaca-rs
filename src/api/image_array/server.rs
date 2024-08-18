use super::{ImageArray, ImageBytesMetadata, COLOUR_AXIS, IMAGE_BYTES_TYPE};
use crate::api::{ImageArrayRank, ImageElementType, TransmissionElementType};
use crate::server::Response;
use crate::ASCOMResult;
use bytemuck::{bytes_of, Zeroable};
use http::header::{HeaderMap, ACCEPT, CONTENT_TYPE};
use serde::{Serialize, Serializer};
use std::mem::size_of;

pub(crate) struct ImageBytesResponse(pub(crate) ImageArray);

impl Response for ASCOMResult<ImageBytesResponse> {
    fn into_axum(
        self,
        transaction: crate::server::ResponseTransaction,
    ) -> axum::response::Response {
        use axum::response::IntoResponse;

        let mut metadata = ImageBytesMetadata {
            metadata_version: 1,
            data_start: i32::try_from(size_of::<ImageBytesMetadata>())
                .expect("internal error: metadata size is too large"),
            client_transaction_id: transaction.client_transaction_id,
            server_transaction_id: Some(transaction.server_transaction_id),
            ..Zeroable::zeroed()
        };
        let bytes = match &self {
            Ok(ImageBytesResponse(img_array)) => {
                metadata.image_element_type = ImageElementType::I32.into();
                metadata.transmission_element_type = img_array.transmission_element_type.into();
                let dims = <[_; 3]>::from(img_array.dim())
                    .map(|dim| i32::try_from(dim).expect("dimension is too large"));
                metadata.dimension_1 = dims[0];
                metadata.dimension_2 = dims[1];
                metadata.rank = match dims[2] {
                    1_i32 => ImageArrayRank::Rank2,
                    n => {
                        metadata.dimension_3 = n;
                        ImageArrayRank::Rank3
                    }
                }
                .into();
                let mut bytes = Vec::with_capacity(
                    size_of::<ImageBytesMetadata>()
                        + img_array.len()
                            * match img_array.transmission_element_type {
                                TransmissionElementType::I32 => size_of::<i32>(),
                                TransmissionElementType::U8 => size_of::<u8>(),
                                TransmissionElementType::I16 => size_of::<i16>(),
                                TransmissionElementType::U16 => size_of::<u16>(),
                            },
                );
                bytes.extend_from_slice(bytes_of(&metadata));
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                match img_array.transmission_element_type {
                    TransmissionElementType::I32 => {
                        bytes.extend(img_array.iter().flat_map(|&i| i.to_le_bytes()));
                    }
                    TransmissionElementType::U8 => {
                        bytes.extend(img_array.iter().map(|&i| i as u8));
                    }
                    TransmissionElementType::I16 => {
                        bytes.extend(img_array.iter().flat_map(|&i| (i as i16).to_le_bytes()));
                    }
                    TransmissionElementType::U16 => {
                        bytes.extend(img_array.iter().flat_map(|&i| (i as u16).to_le_bytes()));
                    }
                }
                bytes
            }
            Err(err) => {
                metadata.error_number = err.code.raw().into();
                let mut bytes =
                    Vec::with_capacity(size_of::<ImageBytesMetadata>() + err.message.len());
                bytes.extend_from_slice(bytes_of(&metadata));
                bytes.extend_from_slice(err.message.as_bytes());
                bytes
            }
        };
        ([(CONTENT_TYPE, IMAGE_BYTES_TYPE)], bytes).into_response()
    }
}

impl Serialize for ImageArray {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct JsonImageArray<'img> {
            #[serde(rename = "Type")]
            type_: ImageElementType,
            rank: ImageArrayRank,
            value: Value<'img>,
        }

        #[derive(Serialize)]
        #[serde(untagged)]
        enum Value<'img> {
            Rank2(#[serde(with = "serde_ndim")] ndarray::ArrayView2<'img, i32>),
            Rank3(#[serde(with = "serde_ndim")] ndarray::ArrayView3<'img, i32>),
        }

        let view = self.data.view();

        JsonImageArray {
            type_: ImageElementType::I32,
            rank: self.rank(),
            value: match self.rank() {
                ImageArrayRank::Rank2 => Value::Rank2(view.remove_axis(COLOUR_AXIS)),
                ImageArrayRank::Rank3 => Value::Rank3(view),
            },
        }
        .serialize(serializer)
    }
}

impl ImageArray {
    pub(crate) fn is_accepted(headers: &HeaderMap) -> bool {
        use mediatype::{MediaType, MediaTypeList};

        const MEDIA_TYPE: MediaType<'static> = MediaType::new(
            mediatype::names::APPLICATION,
            mediatype::Name::new_unchecked("imagebytes"),
        );

        headers
            .get_all(ACCEPT)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(MediaTypeList::new)
            .filter_map(Result::ok)
            .any(|media_type| media_type.essence() == MEDIA_TYPE)
    }
}
