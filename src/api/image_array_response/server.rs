use super::{ImageArrayResponse, ImageBytesMetadata, COLOUR_AXIS, IMAGE_BYTES_TYPE};
use crate::api::{ImageArrayResponseRank, ImageArrayResponseType};
use crate::server::Response;
use crate::ASCOMResult;
use bytemuck::Zeroable;
use serde::{Serialize, Serializer};

pub(crate) struct ImageBytesResponse(pub(crate) ImageArrayResponse);

impl Response for ASCOMResult<ImageBytesResponse> {
    fn into_axum(
        self,
        transaction: crate::server::ResponseTransaction,
    ) -> axum::response::Response {
        use axum::response::IntoResponse;

        let mut metadata = ImageBytesMetadata {
            metadata_version: 1,
            data_start: i32::try_from(std::mem::size_of::<ImageBytesMetadata>())
                .expect("internal error: metadata size is too large"),
            client_transaction_id: transaction.client_transaction_id,
            server_transaction_id: Some(transaction.server_transaction_id),
            ..Zeroable::zeroed()
        };
        let data = match &self {
            Ok(ImageBytesResponse(ImageArrayResponse { data })) => {
                metadata.image_element_type = ImageArrayResponseType::Integer as i32;
                metadata.transmission_element_type = ImageArrayResponseType::Integer as i32;
                let dims = {
                    let (dim0, dim1, dim2) = data.dim();
                    [dim0, dim1, dim2]
                        .map(|dim| i32::try_from(dim).expect("dimension is too large"))
                };
                metadata.dimension_1 = dims[0];
                metadata.dimension_2 = dims[1];
                metadata.rank = match dims[2] {
                    1_i32 => 2_i32,
                    n => {
                        metadata.dimension_3 = n;
                        3_i32
                    }
                };
                bytemuck::cast_slice(
                    data.as_slice()
                        .expect("internal arrays should always be in standard layout"),
                )
            }
            Err(err) => {
                metadata.error_number = err.code.raw().into();
                err.message.as_bytes()
            }
        };
        let mut bytes = Vec::with_capacity(std::mem::size_of::<ImageBytesMetadata>() + data.len());
        bytes.extend_from_slice(bytemuck::bytes_of(&metadata));
        bytes.extend_from_slice(data);
        (
            [(axum::http::header::CONTENT_TYPE, IMAGE_BYTES_TYPE)],
            bytes,
        )
            .into_response()
    }
}

impl Serialize for ImageArrayResponse {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct JsonImageArrayResponse<'img> {
            #[serde(rename = "Type")]
            type_: ImageArrayResponseType,
            rank: ImageArrayResponseRank,
            value: Value<'img>,
        }

        #[derive(Serialize)]
        #[serde(untagged)]
        enum Value<'img> {
            Rank2(#[serde(with = "serde_ndim")] ndarray::ArrayView2<'img, i32>),
            Rank3(#[serde(with = "serde_ndim")] ndarray::ArrayView3<'img, i32>),
        }

        let view = self.data.view();

        JsonImageArrayResponse {
            type_: ImageArrayResponseType::Integer,
            rank: self.rank(),
            value: match self.rank() {
                ImageArrayResponseRank::Rank2 => Value::Rank2(view.remove_axis(COLOUR_AXIS)),
                ImageArrayResponseRank::Rank3 => Value::Rank3(view),
            },
        }
        .serialize(serializer)
    }
}

impl ImageArrayResponse {
    pub(crate) fn is_accepted(headers: &axum::headers::HeaderMap) -> bool {
        use mediatype::{MediaType, MediaTypeList};

        const MEDIA_TYPE: MediaType<'static> = MediaType::new(
            mediatype::names::APPLICATION,
            mediatype::Name::new_unchecked("imagebytes"),
        );

        headers
            .get_all(axum::http::header::ACCEPT)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(MediaTypeList::new)
            .filter_map(std::result::Result::ok)
            .any(|media_type| media_type.essence() == MEDIA_TYPE)
    }
}
