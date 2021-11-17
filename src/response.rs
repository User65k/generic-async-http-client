use serde::de::DeserializeOwned;
use crate::{Error, HeaderName, HeaderValue, imp};
use std::convert::TryInto;

/// The response of a webserver.
/// Headers and Status are available from the start,
/// the body must be polled/awaited again
pub struct Response(pub(crate) imp::Resp);
impl Response {
    /// Return the status code
    pub fn status_code(&self) -> u16 {
        self.0.status()
    }
    /// Return the status as string
    pub fn status(&self) -> &str {
        self.0.status_str()
    }
    /// Return the Body as some type deserialized from JSON
    pub async fn json<D: DeserializeOwned>(&mut self) -> Result<D, Error> {
        self.0.json().await
    }
    /// Return the whole Body as Bytes
    pub async fn content(&mut self) -> Result<Vec<u8>, Error> {
        self.0.bytes().await
    }
    /// Return the whole Body as String
    pub async fn text(&mut self) -> Result<String, Error> {
        self.0.string().await
    }
    /// If there are multiple values associated with the key, then the first one is returned.
    pub fn header(
        &self,
        name: impl TryInto<HeaderName, Error = imp::Error>,
    ) -> Option<&HeaderValue> {
        match name.try_into() {
            Err(_) => None,
            Ok(name) => self.0.get_header(name.into()).map(|v| v.into()),
        }
    }
    /// Each key will be yielded once per associated value. So, if a key has 3 associated values, it will be yielded 3 times.
    pub fn headers(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.0.header_iter().map(|(n, v)| (n.into(), v.into()))
    }
    /*
    TODO cookie
    TODO encoding
    TODO raw (impl Read and or Stream)
    */
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h: Vec<(&HeaderName, &HeaderValue)> = self.headers().collect();
        write!(f, "HTTP {} Header: {:?}", self.status_code(), h)
    }
}