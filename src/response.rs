use crate::{imp, Error, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;
use std::convert::TryInto;

/// The response of a webserver.
/// Headers and Status are available from the start,
/// the body must be polled/awaited again
///
/// Depending on the chosen implementation, `Response` implements `Into<http_types::Response>` or `Into<hyper::Response>`.
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
        Ok(self.0.json().await?)
    }
    /// Return the whole Body as Bytes
    pub async fn content(&mut self) -> Result<Vec<u8>, Error> {
        Ok(self.0.bytes().await?)
    }
    /// Return the whole Body as String
    pub async fn text(&mut self) -> Result<String, Error> {
        Ok(self.0.string().await?)
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
    /// return an error if `name` is not a valid header name
    pub fn all_header(
        &self,
        name: impl TryInto<HeaderName, Error = imp::Error>,
    ) -> Result<impl Iterator<Item = &HeaderValue>, Error> {
        let name: HeaderName = name.try_into()?;
        Ok(self.0.get_headers(name.into()).map(|v| v.into()))
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

pub trait Responses {
    fn status(&self) -> u16;
    fn status_str(&self) -> &'static str;
    async fn json<D: DeserializeOwned>(&mut self) -> Result<D, imp::Error>;
    async fn bytes(&mut self) -> Result<Vec<u8>, imp::Error>;
    async fn string(&mut self) -> Result<String, imp::Error>;
    fn get_header(&self, name: imp::HeaderName) -> Option<&imp::HeaderValue>;
    fn get_headers(&self, name: imp::HeaderName) -> impl Iterator<Item = &imp::HeaderValue>;
    fn header_iter(&self) -> impl Iterator<Item = (&imp::HeaderName, &imp::HeaderValue)>;
}
