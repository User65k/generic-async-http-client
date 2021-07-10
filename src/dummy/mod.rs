use serde::Serialize;
use serde::de::DeserializeOwned;
use std::convert::TryFrom;

//TODO maybe add some mock stuff for testing

#[derive(Debug)]
pub struct Req {
}
pub struct Resp {
}
pub struct Body(Vec<u8>);
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct HeaderName(String);
#[derive(Debug)]
pub struct HeaderValue(Vec<u8>);

impl Req {
    pub fn get(uri: &str) -> Req {
        Req::new("GET",uri).unwrap()
    }
    pub fn post(uri: &str) -> Req {
        Req::new("POST",uri).unwrap()
    }
    pub fn put(uri: &str) -> Req {
        Req::new("PUT",uri).unwrap()
    }
    pub fn delete(uri: &str) -> Req {
        Req::new("DELETE",uri).unwrap()
    }
    pub fn head(uri: &str) -> Req {
        Req::new("HEAD",uri).unwrap()
    }
    pub fn options(uri: &str) -> Req {
        Req::new("OPTIONS",uri).unwrap()
    }
    pub fn new(meth: &str, uri: &str) -> Result<Req, Error> {
        log::debug!("{} {}", meth, uri);
        Ok(Req {
        })
    }
    pub async fn send_request(self) -> Result<Resp, Error> {
        eprintln!("No HTTP backend was selected");
        println!("No HTTP backend was selected");
        Err(Error{})
    }
    pub fn json<T: Serialize + ?Sized>(&mut self, _json: &T) -> Result<(), Error> {
        Ok(())
    }
    pub fn form<T: Serialize + ?Sized>(&mut self, _data: &T) -> Result<(), Error> {
        Ok(())
    }
    pub fn query<T: Serialize + ?Sized>(&mut self, _query: &T) -> Result<(), Error> {
        Ok(())
    }
    pub fn body<B: Into<Body>>(&mut self, _body: B) -> Result<(), Error> {
        Ok(())
    }
    pub fn set_header(&mut self, _name: HeaderName, _values: HeaderValue) -> Result<(), Error> {
        Ok(())
    }
    pub fn add_header(&mut self, _name: HeaderName, _values: HeaderValue) -> Result<(), Error> {
        Ok(())
    }
}
impl Resp {
    pub fn status(&self) -> u16 {
        500
    }
    pub fn status_str(&self) -> &'static str {
        "not implemented"
    }
    pub async fn json<D: DeserializeOwned>(&mut self) -> Result<D, Error> {
        Err(Error{})
    }
    pub async fn bytes(&mut self) -> Result<Vec<u8>, Error> {
        Err(Error{})
    }
    pub async fn string(&mut self) -> Result<String, Error> {
        Err(Error{})
    }
    pub fn get_header(&self, _name: HeaderName) -> Option<&HeaderValue> {
        None
    }
    pub fn header_iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        vec!().into_iter()
    }
}

#[derive(Debug)]
pub struct Error {}
impl std::error::Error for Error {}
use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not implemented")
    }
}

impl From<String> for Body {
    #[inline]
    fn from(t: String) -> Self {
        Body(t.as_bytes().to_vec())
    }
}
impl From<Vec<u8>> for Body {
    #[inline]
    fn from(t: Vec<u8>) -> Self {
        Body(t)
    }
}
impl From<&'static [u8]> for Body {
    #[inline]
    fn from(t: &'static [u8]) -> Self {
        Body(t.to_vec())
    }
}
impl From<&'static str> for Body {
    #[inline]
    fn from(t: &'static str) -> Self {
        Body(t.as_bytes().to_vec())
    }
}
impl<'a> TryFrom<&'a str> for HeaderValue {
    type Error = Error;
    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        Ok(HeaderValue(t.as_bytes().to_vec()))
    }
}
