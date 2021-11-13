use crate::tcp::Stream;
use async_std::io;
pub use http_types::{
    headers::{HeaderName, HeaderValue, HeaderValues, Iter as HttpHeaderIter, ToHeaderValues},
    Body,
};
use http_types::{Method, Request, Response};
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug)]
pub struct Req {
    req: Request,
}
pub struct Resp {
    resp: Response,
}

impl Req {
    pub fn get(uri: &str) -> Req {
        Self::init(Method::Get, uri)
    }
    pub fn post(uri: &str) -> Req {
        Self::init(Method::Post, uri)
    }
    pub fn put(uri: &str) -> Req {
        Self::init(Method::Put, uri)
    }
    pub fn delete(uri: &str) -> Req {
        Self::init(Method::Delete, uri)
    }
    pub fn head(uri: &str) -> Req {
        Self::init(Method::Head, uri)
    }
    pub fn options(uri: &str) -> Req {
        Self::init(Method::Options, uri)
    }
    pub fn new(meth: &str, uri: &str) -> Result<Req, Error> {
        Ok(Self::init(Method::from_str(meth)?, uri))
    }
    fn init(method: Method, uri: &str) -> Req {
        let req = Request::new(method, uri);
        Req { req }
    }
    pub async fn send_request(self) -> Result<Resp, Error> {
        let tls = match self.req.url().scheme() {
            "https" => true,
            "http" => false,
            _ => return Err(Error::Scheme),
        };

        let host = match self.req.host() {
            None => return Err(Error::UndefinedHost),
            Some(host) => host,
        };
        let port = match self.req.url().port() {
            None => {
                if tls {
                    443
                } else {
                    80
                }
            }
            Some(port) => port,
        };
        let transport = Stream::connect(host, port, tls).await?;

        let resp = async_h1::connect(transport, self.req).await?;
        Ok(Resp { resp })
    }
    pub fn json<T: Serialize + ?Sized>(&mut self, json: &T) -> Result<(), Error> {
        self.req.set_body(Body::from_json(&json)?);
        Ok(())
    }
    pub fn form<T: Serialize + ?Sized>(&mut self, data: &T) -> Result<(), Error> {
        self.req.set_body(Body::from_form(&data)?);
        Ok(())
    }
    pub fn query<T: Serialize + ?Sized>(&mut self, query: &T) -> Result<(), Error> {
        self.req.set_query(&query)?;
        Ok(())
    }
    pub fn body<B: Into<Body>>(&mut self, body: B) -> Result<(), Error> {
        self.req.set_body(body);
        Ok(())
    }
    pub fn set_header(&mut self, name: HeaderName, values: HeaderValue) -> Result<(), Error> {
        self.req.insert_header(name, values);
        Ok(())
    }
    pub fn add_header(&mut self, name: HeaderName, values: HeaderValue) -> Result<(), Error> {
        self.req.append_header(name, values);
        Ok(())
    }
}
use serde::de::DeserializeOwned;
impl Resp {
    pub fn status(&self) -> u16 {
        self.resp.status().into()
    }
    pub fn status_str(&self) -> &'static str {
        self.resp.status().canonical_reason()
    }
    pub async fn json<D: DeserializeOwned>(&mut self) -> Result<D, Error> {
        Ok(self.resp.body_json().await?)
    }
    pub async fn bytes(&mut self) -> Result<Vec<u8>, Error> {
        Ok(self.resp.body_bytes().await?)
    }
    pub async fn string(&mut self) -> Result<String, Error> {
        Ok(self.resp.body_string().await?)
    }
    pub fn get_header(&self, name: HeaderName) -> Option<&HeaderValue> {
        self.resp.header(name).and_then(|v| v.iter().next())
    }
    pub fn header_iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        HeaderIter::new(self.resp.iter())
    }
}
/// unroll the grouped headers
pub struct HeaderIter<'a> {
    iter: HttpHeaderIter<'a>,
    current: Option<(&'a HeaderName, &'a HeaderValues)>,
    index: usize,
}
impl HeaderIter<'_> {
    pub fn new(iter: HttpHeaderIter) -> HeaderIter {
        HeaderIter {
            iter,
            current: None,
            index: 0,
        }
    }
}
impl<'a> Iterator for HeaderIter<'a> {
    type Item = (&'a HeaderName, &'a HeaderValue);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((n, v)) = self.current {
            self.index += 1;
            if let Some(val) = v.get(self.index) {
                return Some((n, val));
            }
        }

        if let Some((n, v)) = self.iter.next() {
            self.index = 0;
            self.current = Some((n, v));
            return Some((n, v.get(0).expect("header must have at least one value")));
        }
        None
    }
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Http(http_types::Error),
    UndefinedHost,
    Scheme,
}
impl std::error::Error for Error {}
use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<http_types::Error> for Error {
    fn from(e: http_types::Error) -> Self {
        Self::Http(e)
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
