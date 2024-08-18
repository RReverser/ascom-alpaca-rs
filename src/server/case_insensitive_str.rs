use bytemuck::{TransparentWrapper, TransparentWrapperAlloc};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

#[derive(Serialize, TransparentWrapper)]
#[serde(transparent)]
#[repr(transparent)]
pub(crate) struct CaseInsensitiveStr(str);

impl AsRef<CaseInsensitiveStr> for str {
    fn as_ref(&self) -> &CaseInsensitiveStr {
        TransparentWrapper::wrap_ref(self)
    }
}

impl From<Box<str>> for Box<CaseInsensitiveStr> {
    fn from(s: Box<str>) -> Self {
        TransparentWrapperAlloc::wrap_box(s)
    }
}

impl<'de> Deserialize<'de> for Box<CaseInsensitiveStr> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <Box<str>>::deserialize(deserializer).map(Into::into)
    }
}

impl Debug for CaseInsensitiveStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for CaseInsensitiveStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for CaseInsensitiveStr {}

impl Hash for CaseInsensitiveStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for b in self.0.as_bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
        state.write_u8(0xff);
    }
}
