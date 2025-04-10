use crate::tcp::Stream;
use async_std::io;
pub use http_types::{
    headers::{HeaderName, HeaderValue},
    Body,
};
use http_types::{
    headers::{HeaderValues, Iter as HttpHeaderIter},
    Method, Request, Response, Url,
};
use serde::Serialize;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

#[derive(Debug)]
pub struct Req {
    req: Request,
}

impl<M, U> TryFrom<(M, U)> for crate::Request
where
    Method: TryFrom<M>,
    Url: TryFrom<U>,
    <Url as TryFrom<U>>::Error: std::fmt::Debug,
{
    type Error = <Method as TryFrom<M>>::Error;

    fn try_from(value: (M, U)) -> Result<Self, Self::Error> {
        let req = Request::new(value.0.try_into()?, value.1);
        Ok(crate::Request(Req { req }))
    }
}
impl Req {
    fn init(method: Method, uri: &str) -> Req {
        let req = Request::new(method, uri);
        Req { req }
    }
}
impl crate::request::Requests for Req {
    fn get(uri: &str) -> Req {
        Self::init(Method::Get, uri)
    }
    fn post(uri: &str) -> Req {
        Self::init(Method::Post, uri)
    }
    fn put(uri: &str) -> Req {
        Self::init(Method::Put, uri)
    }
    fn delete(uri: &str) -> Req {
        Self::init(Method::Delete, uri)
    }
    fn head(uri: &str) -> Req {
        Self::init(Method::Head, uri)
    }
    fn options(uri: &str) -> Req {
        Self::init(Method::Options, uri)
    }
    fn new(meth: &str, uri: &str) -> Result<Req, Error> {
        Ok(Self::init(Method::from_str(meth)?, uri))
    }
    async fn send_request(self) -> Result<crate::Response, Error> {
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
        //TODO implement clonable connection (RW) like FCGI
        //check connection headers, connect method and upgrades
        //free slot once body is consumed

        #[cfg(not(all(feature = "mock_tests", test)))]
        return Ok(crate::Response(Resp { resp }));
        #[cfg(all(feature = "mock_tests", test))]
        return Ok(crate::Response(Resp::Real(not_mocked::Resp { resp })));
    }
    fn json<T: Serialize + ?Sized>(&mut self, json: &T) -> Result<(), Error> {
        self.req.set_body(Body::from_json(&json)?);
        Ok(())
    }
    fn form<T: Serialize + ?Sized>(&mut self, data: &T) -> Result<(), Error> {
        self.req.set_body(Body::from_form(&data)?);
        Ok(())
    }
    fn query<T: Serialize + ?Sized>(&mut self, query: &T) -> Result<(), Error> {
        self.req.set_query(&query)?;
        Ok(())
    }
    fn body<B: Into<Body>>(&mut self, body: B) -> Result<(), Error> {
        self.req.set_body(body);
        Ok(())
    }
    fn set_header(&mut self, name: HeaderName, values: HeaderValue) -> Result<(), Error> {
        self.req.insert_header(name, values);
        Ok(())
    }
    fn add_header(&mut self, name: HeaderName, values: HeaderValue) -> Result<(), Error> {
        self.req.append_header(name, values);
        Ok(())
    }
}
mod not_mocked {
    use super::*;
    use serde::de::DeserializeOwned;
    pub struct Resp {
        pub(super) resp: Response,
    }
    impl crate::response::Responses for Resp {
        fn status(&self) -> u16 {
            self.resp.status().into()
        }
        fn status_str(&self) -> &'static str {
            self.resp.status().canonical_reason()
        }
        async fn json<D: DeserializeOwned>(&mut self) -> Result<D, Error> {
            Ok(self.resp.body_json().await?)
        }
        async fn bytes(&mut self) -> Result<Vec<u8>, Error> {
            Ok(self.resp.body_bytes().await?)
        }
        async fn string(&mut self) -> Result<String, Error> {
            Ok(self.resp.body_string().await?)
        }
        fn get_header(&self, name: HeaderName) -> Option<&HeaderValue> {
            self.resp.header(name).and_then(|v| v.iter().next())
        }
        fn header_iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
            HeaderIter::new(self.resp.iter())
        }
        fn get_headers(&self, name: super::HeaderName) -> impl Iterator<Item = &HeaderValue> {
            if let Some(h) = self.resp.header(name) {
                Vec::from_iter(h.iter()).into_iter()
            } else {
                vec![].into_iter()
            }
        }
    }
}
#[cfg(not(all(feature = "mock_tests", test)))]
pub use not_mocked::Resp;
#[cfg(all(feature = "mock_tests", test))]
pub type Resp = crate::mock::Resp<not_mocked::Resp>;
#[cfg(all(feature = "mock_tests", test))]
mod mocked {
    use super::*;
    use async_std::task::block_on;
    impl From<serde_json::Error> for Error {
        fn from(e: serde_json::Error) -> Self {
            panic!("test with invalid json {}", e);
        }
    }
    impl crate::mock::MockedRequest for Req {
        /// on error, return full body
        fn assert_body_bytes(&mut self, should_be: &[u8]) -> Result<(), Vec<u8>> {
            let is = block_on(self.req.body_bytes()).unwrap_or_default();
            if is != should_be {
                Err(is.clone())
            } else {
                Ok(())
            }
        }
        fn get_headers(&self, name: &str) -> Option<Vec<crate::mock::MockHeaderValue>> {
            let name = HeaderName::from_string(name.to_string()).unwrap();
            let hm = self.req.header(name)?;
            Some(hm.iter().cloned().map(|v| v.into()).collect())
        }
        fn endpoint(&self) -> crate::mock::Endpoint {
            (self.req.method().to_string(), self.req.url().to_string())
        }
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
        write!(f, "{:?}", self)
    }
}
impl From<Error> for crate::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::Io(error) => Self::Io(error),
            e => Self::Other(e),
        }
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
