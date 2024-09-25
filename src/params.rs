#[derive(Debug, Clone, Copy)]
pub(crate) enum Method {
    Get,
    Put,
}

impl From<Method> for reqwest::Method {
    fn from(method: Method) -> Self {
        match method {
            Method::Get => Self::GET,
            Method::Put => Self::PUT,
        }
    }
}

pub(crate) struct ActionParams<T> {
    pub(crate) action: &'static str,
    pub(crate) method: Method,
    pub(crate) params: T,
}

pub(crate) trait Action: Sized + Send {
    #[cfg(feature = "server")]
    fn from_parts(
        action: &str,
        params: &mut crate::server::ActionParams,
    ) -> crate::server::Result<Option<Self>>;

    #[cfg(feature = "client")]
    fn into_parts(self) -> ActionParams<impl serde::Serialize + Send>;
}
