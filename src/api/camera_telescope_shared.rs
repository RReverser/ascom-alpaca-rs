use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// The direction in which the guide-rate motion is to be made.
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
pub enum GuideDirection {
    /// North (+ declination/altitude).
    North = 0,

    /// South (- declination/altitude).
    South = 1,

    /// East (+ right ascension/azimuth).
    East = 2,

    /// West (- right ascension/azimuth).
    West = 3,
}

#[cfg(feature = "client")]
pub(super) trait ConvertConvenienceProp {
    type Inner;
    type Arr;

    fn from_arr(arr: Self::Arr) -> Self;
    fn into_arr(self) -> Self::Arr;
}

#[cfg(feature = "client")]
const _: () = {
    impl<T> ConvertConvenienceProp for (T, T) {
        type Inner = T;
        type Arr = [T; 2];

        fn from_arr(arr: Self::Arr) -> Self {
            arr.into()
        }

        fn into_arr(self) -> Self::Arr {
            self.into()
        }
    }

    impl<T, const N: usize> ConvertConvenienceProp for [T; N] {
        type Inner = T;
        type Arr = [T; N];

        fn from_arr(arr: Self::Arr) -> Self {
            arr
        }

        fn into_arr(self) -> Self::Arr {
            self
        }
    }

    impl<T> ConvertConvenienceProp for std::ops::RangeInclusive<T> {
        type Inner = T;
        type Arr = [T; 2];

        fn from_arr([start, end]: Self::Arr) -> Self {
            start..=end
        }

        fn into_arr(self) -> Self::Arr {
            self.into_inner().into()
        }
    }
};

#[cfg_attr(not(feature = "client"), expect(unused_macro_rules))]
macro_rules! convenience_props {
    (@prop
        $trait_name:ident
        $(#[doc = $doc:literal])+
        #[
            $(#[doc = $with_set_doc:literal])+
            set
        ]
        $prop:ident($($sub_prop:ident),+) : $ty:ty
    ) => {
        convenience_props!(@prop
            $trait_name
            $(#[doc = $doc])+
            $prop($($sub_prop),+) : $ty
        );

        paste::paste! {
            $(#[doc = $with_set_doc])+
            ///
            /// This is an aggregation of following methods, see their docs for more details:
            $(
                #[doc = " - [`set_" $sub_prop "`](" $trait_name "::set_" $sub_prop ")"]
            )+
            pub async fn [<set_ $prop>](&self, $prop: $ty) -> ASCOMResult<()> {
                let [$($sub_prop),+] = $crate::api::camera_telescope_shared::ConvertConvenienceProp::into_arr($prop);
                tokio::try_join!(
                    $(self.[<set_ $sub_prop>]($sub_prop),)+
                ).map(|_| ())
            }
        }
    };

    (@prop
        $trait_name:ident
        $(#[doc = $doc:literal])+
        $prop:ident($($sub_prop:ident),+) : $ty:ty
    ) => {
        $(#[doc = $doc])+
        ///
        /// This is an aggregation of following methods, see their docs for more details:
        $(
            #[doc = concat!(" - [`", stringify!($sub_prop), "`](", stringify!($trait_name), "::", stringify!($sub_prop), ")")]
        )+
        pub async fn $prop(&self) -> ASCOMResult<$ty> {
            tokio::try_join!($(self.$sub_prop(),)+)
            .map(|tuple| $crate::api::camera_telescope_shared::ConvertConvenienceProp::from_arr(tuple.into()))
        }
    };

    ($trait_name:ident { $(
        $(# $attr:tt)*
        $prop:ident($($sub_prop:ident),*) : $ty:ty,
    )* }) => {
        /// Convenience methods for the client to get/set related properties together.
        #[cfg(feature = "client")]
        impl dyn $trait_name {
            $(
                convenience_props!(@prop
                    $trait_name
                    $(# $attr)*
                    $prop($($sub_prop),*) : $ty
                );
            )*
        }
    };
}
