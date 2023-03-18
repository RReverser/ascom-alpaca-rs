use crate::params::{ASCOMParam, OpaqueParams};
use std::hash::Hash;

impl<ParamStr: ?Sized + Hash + Eq> OpaqueParams<ParamStr>
where
    Box<ParamStr>: From<Box<str>>,
{
    pub(crate) fn insert<T: ASCOMParam>(&mut self, name: &str, value: T) {
        let prev_value = self
            .0
            .insert(Box::<str>::from(name).into(), value.to_string());
        debug_assert!(prev_value.is_none());
    }
}
