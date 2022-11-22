#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ImageArrayResponseRank {
    Mono = 2,
    Rgb = 3,
}

pub trait ImageArrayResponseNumber: Serialize {
    const TYPE: ImageArrayResponseType;
}

impl ImageArrayResponseNumber for i16 {
    const TYPE: ImageArrayResponseType = ImageArrayResponseType::Short;
}

impl ImageArrayResponseNumber for i32 {
    const TYPE: ImageArrayResponseType = ImageArrayResponseType::Integer;
}

impl ImageArrayResponseNumber for f64 {
    const TYPE: ImageArrayResponseType = ImageArrayResponseType::Double;
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ImageArrayTypedResponse<T: ImageArrayResponseNumber> {
    pub rank: ImageArrayResponseRank,
    pub height: u32,
    pub flat_data: Vec<T>,
}

impl<T: ImageArrayResponseNumber> ImageArrayTypedResponse<T> {
    #[auto_enums::auto_enum(serde::Serialize)]
    fn value_as_serialize(&self) -> impl '_ + Serialize {
        #[derive(Serialize)]
        struct IterSerialize<I: Iterator + Clone>(#[serde(with = "serde_iter::seq")] I)
        where
            I::Item: Serialize;

        let rows_iter = self.flat_data.chunks_exact(self.height as usize);
        match self.rank {
            ImageArrayResponseRank::Mono => IterSerialize(rows_iter),
            ImageArrayResponseRank::Rgb => IterSerialize(rows_iter.map(|row| {
                IterSerialize(
                    // TODO: use array_chunks when stabilized
                    row.chunks_exact(3).map(|rgb| {
                        #[allow(clippy::unwrap_used)]
                        <&[T; 3]>::try_from(rgb).unwrap()
                    }),
                )
            })),
        }
    }
}

impl<T: ImageArrayResponseNumber> Serialize for ImageArrayTypedResponse<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Repr<Value> {
            #[serde(rename = "Type")]
            type_: ImageArrayResponseType,
            rank: ImageArrayResponseRank,
            value: Value,
        }

        Repr {
            type_: T::TYPE,
            rank: self.rank,
            value: self.value_as_serialize(),
        }
        .serialize(serializer)
    }
}

pub type ImageArrayResponse = ImageArrayTypedResponse<i32>;

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
#[serde(untagged)]
pub enum ImageArrayVariantResponse {
    Short(ImageArrayTypedResponse<i16>),
    Integer(ImageArrayTypedResponse<i32>),
    Double(ImageArrayTypedResponse<f64>),
}
