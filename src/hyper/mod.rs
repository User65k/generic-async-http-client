use std::str::FromStr;

use serde::Serialize;

pub use hyper::{
    header::{HeaderName, HeaderValue},
    Body,
};
use hyper::{
    header::{InvalidHeaderName, InvalidHeaderValue, CONTENT_TYPE},
    http::{
        method::{InvalidMethod, Method},
        request::Builder,
        uri::{Builder as UriBuilder, InvalidUri, PathAndQuery},
        Error as HTTPError,
    },
    Client, Error as HyperError, Response,
};
use std::mem::take;

mod connector;
use connector::Connector;

#[derive(Debug)]
pub struct Req {
    req: Builder,
    body: Body,
}
pub struct Resp {
    resp: Response<Body>,
}

impl Req {
    pub fn get(uri: &str) -> Req {
        Self::init(Method::GET, uri)
    }
    pub fn post(uri: &str) -> Req {
        Self::init(Method::POST, uri)
    }
    pub fn put(uri: &str) -> Req {
        Self::init(Method::PUT, uri)
    }
    pub fn delete(uri: &str) -> Req {
        Self::init(Method::DELETE, uri)
    }
    pub fn head(uri: &str) -> Req {
        Self::init(Method::HEAD, uri)
    }
    pub fn options(uri: &str) -> Req {
        Self::init(Method::OPTIONS, uri)
    }
    pub fn new(meth: &str, uri: &str) -> Result<Req, Error> {
        Ok(Self::init(Method::from_str(meth)?, uri))
    }
    fn init(method: Method, uri: &str) -> Req {
        let req = Builder::new().method(method).uri(uri);

        Req {
            req,
            body: Body::empty(),
        }
    }
    pub async fn send_request(self) -> Result<Resp, Error> {
        let req = self.req.body(self.body)?;

        let connector = Connector::new();
        let client = Client::builder().build::<_, Body>(connector);

        let resp = client.request(req).await?;
        Ok(Resp { resp })
    }
    pub fn json<T: Serialize + ?Sized>(&mut self, json: &T) -> Result<(), Error> {
        let bytes = serde_json::to_vec(&json)?;
        self.set_header(CONTENT_TYPE, HeaderValue::from_static("application/json"))?;
        self.body = Body::from(bytes);
        Ok(())
    }
    pub fn form<T: Serialize + ?Sized>(&mut self, data: &T) -> Result<(), Error> {
        let query = serde_urlencoded::to_string(data)?;
        let bytes = query.into_bytes();
        self.set_header(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        )?;
        self.body = Body::from(bytes);
        Ok(())
    }
    pub fn query<T: Serialize + ?Sized>(&mut self, query: &T) -> Result<(), Error> {
        let query = serde_qs::to_string(&query)?;
        let old = self.req.uri_ref().expect("no uri");

        let mut p_and_p = String::with_capacity(old.path().len() + query.len() + 1);
        p_and_p.push_str(old.path());
        p_and_p.push('?');
        p_and_p.push_str(&query);

        let path_and_query = PathAndQuery::from_str(&p_and_p)?;

        let new = UriBuilder::new()
            .scheme(old.scheme_str().unwrap())
            .authority(old.authority().unwrap().as_str())
            .path_and_query(path_and_query)
            .build()?;

        self.req = take(&mut self.req).uri(new);
        Ok(())
    }
    pub fn body<B: Into<Body>>(&mut self, body: B) -> Result<(), Error> {
        self.body = body.into();
        Ok(())
    }
    pub fn set_header(&mut self, name: HeaderName, value: HeaderValue) -> Result<(), Error> {
        self.req.headers_mut().map(|hm| hm.insert(name, value));
        Ok(())
    }
    pub fn add_header(&mut self, name: HeaderName, value: HeaderValue) -> Result<(), Error> {
        self.req = take(&mut self.req).header(name, value);
        Ok(())
    }
}
use hyper::body::Buf;
use hyper::body::{aggregate, to_bytes};
use serde::de::DeserializeOwned;
impl Resp {
    pub fn status(&self) -> u16 {
        self.resp.status().as_u16()
    }
    pub fn status_str(&self) -> &'static str {
        self.resp.status().canonical_reason().unwrap_or("")
    }
    pub async fn json<D: DeserializeOwned>(&mut self) -> Result<D, Error> {
        let reader = aggregate(self.resp.body_mut()).await?.reader();
        Ok(serde_json::from_reader(reader)?)
    }
    pub async fn bytes(&mut self) -> Result<Vec<u8>, Error> {
        let b = to_bytes(self.resp.body_mut()).await?;
        Ok(b.to_vec())
    }
    pub async fn string(&mut self) -> Result<String, Error> {
        let b = self.bytes().await?;
        Ok(String::from_utf8_lossy(&b).to_string())
    }
    pub fn get_header(&self, name: HeaderName) -> Option<&HeaderValue> {
        self.resp.headers().get(name)
    }
    pub fn header_iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.resp.headers().into_iter()
    }
}

#[derive(Debug)]
pub enum Error {
    Scheme,
    Http(HTTPError),
    InvalidQueryString(serde_qs::Error),
    InvalidMethod(InvalidMethod),
    Hyper(HyperError),
    Json(serde_json::Error),
    InvalidHeaderValue(InvalidHeaderValue),
    InvalidHeaderName(InvalidHeaderName),
    InvalidUri(InvalidUri),
    Urlencoded(serde_urlencoded::ser::Error),
}
impl std::error::Error for Error {}
use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<serde_urlencoded::ser::Error> for Error {
    fn from(e: serde_urlencoded::ser::Error) -> Self {
        Self::Urlencoded(e)
    }
}
impl From<InvalidUri> for Error {
    fn from(e: InvalidUri) -> Self {
        Self::InvalidUri(e)
    }
}
impl From<InvalidHeaderName> for Error {
    fn from(e: InvalidHeaderName) -> Self {
        Self::InvalidHeaderName(e)
    }
}

impl From<InvalidHeaderValue> for Error {
    fn from(e: InvalidHeaderValue) -> Self {
        Self::InvalidHeaderValue(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<HyperError> for Error {
    fn from(e: HyperError) -> Self {
        Self::Hyper(e)
    }
}
impl From<InvalidMethod> for Error {
    fn from(e: InvalidMethod) -> Self {
        Self::InvalidMethod(e)
    }
}
impl From<HTTPError> for Error {
    fn from(e: HTTPError) -> Self {
        Self::Http(e)
    }
}
impl From<serde_qs::Error> for Error {
    fn from(e: serde_qs::Error) -> Self {
        Self::InvalidQueryString(e)
    }
}
