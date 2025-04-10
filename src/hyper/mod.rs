use std::{
    convert::{Infallible, TryFrom},
    str::FromStr,
};

use serde::Serialize;

pub use hyper::{
    body::Incoming,
    header::{HeaderName, HeaderValue},
};
use hyper::{
    body::{Body as BodyTrait, Bytes, Frame, SizeHint},
    header::{InvalidHeaderName, InvalidHeaderValue, CONTENT_TYPE},
    http::{
        method::{InvalidMethod, Method},
        request::Builder,
        uri::{Builder as UriBuilder, InvalidUri, PathAndQuery, Uri},
        Error as HTTPError,
    },
    Error as HyperError, Request, Response,
};
use std::mem::take;

mod connector;
pub(crate) use connector::HyperClient;

pub(crate) fn get_client() -> HyperClient {
    HyperClient::default()
}

#[derive(Debug)]
pub struct Req {
    req: Builder,
    body: Body,
    pub(crate) client: Option<HyperClient>,
}

impl<M, U> TryFrom<(M, U)> for crate::Request
where
    Method: TryFrom<M>,
    <Method as TryFrom<M>>::Error: Into<HTTPError>,
    Uri: TryFrom<U>,
    <Uri as TryFrom<U>>::Error: Into<HTTPError>,
{
    type Error = Infallible;

    fn try_from(value: (M, U)) -> Result<Self, Self::Error> {
        let req = Builder::new().method(value.0).uri(value.1);

        Ok(crate::Request(Req {
            req,
            body: Body::empty(),
            client: None,
        }))
    }
}
impl Req {
    fn init(method: Method, uri: &str) -> Req {
        let req = Builder::new().method(method).uri(uri);

        Req {
            req,
            body: Body::empty(),
            client: None,
        }
    }
    fn _query(&mut self, query: String) -> Result<(), Error> {
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
}

impl crate::request::Requests for Req {
    fn get(uri: &str) -> Req {
        Self::init(Method::GET, uri)
    }
    fn post(uri: &str) -> Req {
        Self::init(Method::POST, uri)
    }
    fn put(uri: &str) -> Req {
        Self::init(Method::PUT, uri)
    }
    fn delete(uri: &str) -> Req {
        Self::init(Method::DELETE, uri)
    }
    fn head(uri: &str) -> Req {
        Self::init(Method::HEAD, uri)
    }
    fn options(uri: &str) -> Req {
        Self::init(Method::OPTIONS, uri)
    }
    fn new(meth: &str, uri: &str) -> Result<Req, Error> {
        Ok(Self::init(Method::from_str(meth)?, uri))
    }
    async fn send_request(mut self) -> Result<crate::Response, Error> {
        let req = self.req.body(self.body)?;

        let resp = if let Some(mut client) = self.client.take() {
            client.request(req).await?
        } else {
            get_client().request(req).await?
        };

        #[cfg(not(all(feature = "mock_tests", test)))]
        return Ok(crate::Response(Resp { resp }));
        #[cfg(all(feature = "mock_tests", test))]
        return Ok(crate::Response(Resp::Real(not_mocked::Resp { resp })));
    }
    fn json<T: Serialize + ?Sized>(&mut self, json: &T) -> Result<(), Error> {
        let bytes = serde_json::to_string(&json)?;
        self.set_header(CONTENT_TYPE, HeaderValue::from_static("application/json"))?;
        self.body = bytes.into();
        Ok(())
    }
    fn form<T: Serialize + ?Sized>(&mut self, data: &T) -> Result<(), Error> {
        let query = serde_urlencoded::to_string(data)?;
        self.set_header(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        )?;
        self.body = query.into();
        Ok(())
    }
    #[inline]
    fn query<T: Serialize + ?Sized>(&mut self, query: &T) -> Result<(), Error> {
        // codegen trampoline: https://github.com/rust-lang/rust/issues/77960
        self._query(serde_qs::to_string(&query)?)
    }
    fn body<B: Into<Body>>(&mut self, body: B) -> Result<(), Error> {
        self.body = body.into();
        Ok(())
    }
    fn set_header(&mut self, name: HeaderName, value: HeaderValue) -> Result<(), Error> {
        self.req.headers_mut().map(|hm| hm.insert(name, value));
        Ok(())
    }
    fn add_header(&mut self, name: HeaderName, value: HeaderValue) -> Result<(), Error> {
        self.req = take(&mut self.req).header(name, value);
        Ok(())
    }
}
use hyper::body::Buf;
use serde::de::DeserializeOwned;

mod not_mocked {
    use super::*;
    pub struct Resp {
        pub(super) resp: Response<Incoming>,
    }
    impl crate::response::Responses for Resp {
        fn status(&self) -> u16 {
            self.resp.status().as_u16()
        }
        fn status_str(&self) -> &'static str {
            self.resp.status().canonical_reason().unwrap_or("")
        }
        async fn json<D: DeserializeOwned>(&mut self) -> Result<D, Error> {
            let reader = aggregate(self.resp.body_mut()).await?.reader();
            Ok(serde_json::from_reader(reader)?)
        }
        async fn bytes(&mut self) -> Result<Vec<u8>, Error> {
            let mut b = aggregate(self.resp.body_mut()).await?;
            let capacity = b.remaining();
            //Ok(b.copy_to_bytes(capacity).into())
            let mut v = Vec::with_capacity(capacity);
            let ptr = v.spare_capacity_mut().as_mut_ptr();
            let dst = unsafe { std::slice::from_raw_parts_mut(ptr.cast::<u8>(), capacity) };
            b.copy_to_slice(dst);
            unsafe {
                v.set_len(capacity);
            }
            Ok(v)
        }
        async fn string(&mut self) -> Result<String, Error> {
            let b = self.bytes().await?;
            Ok(String::from_utf8_lossy(&b).to_string())
        }
        fn get_header(&self, name: HeaderName) -> Option<&HeaderValue> {
            self.resp.headers().get(name)
        }
        fn get_headers(&self, name: HeaderName) -> impl Iterator<Item = &HeaderValue> {
            self.resp.headers().get_all(name).iter()
        }
        fn header_iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
            self.resp.headers().into_iter()
        }
    }
}

#[cfg(not(all(feature = "mock_tests", test)))]
pub use not_mocked::Resp;
#[cfg(all(feature = "mock_tests", test))]
pub type Resp = crate::mock::Resp<not_mocked::Resp>;

#[cfg(all(feature = "mock_tests", test))]
impl crate::mock::MockedRequest for Req {
    /// on error, return full body
    fn assert_body_bytes(&mut self, should_be: &[u8]) -> Result<(), Vec<u8>> {
        let is = &self.body.0;
        if is != should_be {
            Err(is.clone())
        } else {
            Ok(())
        }
    }
    fn get_headers(&self, name: &str) -> Option<Vec<crate::mock::MockHeaderValue>> {
        let name = HeaderName::from_str(name).unwrap();
        let hm = self
            .req
            .headers_ref()
            .expect("builder should not have errors");
        if !hm.contains_key(&name) {
            return None;
        }
        Some(hm.get_all(name).iter().cloned().map(|v| v.into()).collect())
    }
    fn endpoint(&self) -> crate::mock::Endpoint {
        let uri = self
            .req
            .uri_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();
        let meth = self
            .req
            .method_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();
        (meth, uri)
    }
}

//(fragmented) memory returned by aggregate
struct FracturedBuf(std::collections::VecDeque<Bytes>);
impl Buf for FracturedBuf {
    fn remaining(&self) -> usize {
        self.0.iter().map(|buf| buf.remaining()).sum()
    }
    fn chunk(&self) -> &[u8] {
        self.0.front().map(Buf::chunk).unwrap_or_default()
    }
    fn advance(&mut self, mut cnt: usize) {
        let bufs = &mut self.0;
        while cnt > 0 {
            if let Some(front) = bufs.front_mut() {
                let rem = front.remaining();
                if rem > cnt {
                    front.advance(cnt);
                    return;
                } else {
                    front.advance(rem);
                    cnt -= rem;
                }
            } else {
                //no data -> panic?
                return;
            }
            bufs.pop_front();
        }
    }
}
/// Helper for aggregate function. Polls a single frame from an incoming body
struct Framed<'a>(&'a mut Incoming);

impl futures::Future for Framed<'_> {
    type Output = Option<Result<hyper::body::Frame<Bytes>, hyper::Error>>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        ctx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.0).poll_frame(ctx)
    }
}
/// read an incoming body to (fragmented) memory
async fn aggregate(body: &mut Incoming) -> Result<FracturedBuf, Error> {
    let mut v = std::collections::VecDeque::new();
    while let Some(f) = Framed(body).await {
        if let Ok(d) = f?.into_data() {
            v.push_back(d);
        }
    }
    Ok(FracturedBuf(v))
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
    Io(std::io::Error),
}
impl std::error::Error for Error {}
use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Scheme => write!(f, "Scheme"),
            Error::Http(i) => write!(f, "{}", i),
            Error::InvalidQueryString(i) => write!(f, "{}", i),
            Error::InvalidMethod(i) => write!(f, "{}", i),
            Error::Hyper(i) => write!(f, "{}", i),
            Error::Json(i) => write!(f, "{}", i),
            Error::InvalidHeaderValue(i) => write!(f, "{}", i),
            Error::InvalidHeaderName(i) => write!(f, "{}", i),
            Error::InvalidUri(i) => write!(f, "{}", i),
            Error::Urlencoded(i) => write!(f, "{}", i),
            Error::Io(i) => write!(f, "{}", i),
        }
    }
}
impl From<Error> for crate::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::Io(error) => Self::Io(error),
            Error::Hyper(h) => {
                //It might be an IO error. If so, return it as such
                if let Some(io) = std::error::Error::source(&h)
                    .and_then(|err| err.downcast_ref::<std::io::Error>())
                {
                    let io_e = if let Some(code) = io.raw_os_error() {
                        std::io::Error::from_raw_os_error(code)
                    //}else if let Some(error) = io.into_inner() {
                    //    std::io::Error::new(io.kind(), error)
                    } else {
                        io.kind().into()
                    };
                    Self::Io(io_e)
                } else {
                    Self::Other(Error::Hyper(h))
                }
            }
            e => Self::Other(e),
        }
    }
}
//connect_to_uri
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
//Req::form
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
//TryFrom<> for HeaderName
impl From<InvalidHeaderName> for Error {
    fn from(e: InvalidHeaderName) -> Self {
        Self::InvalidHeaderName(e)
    }
}
//TryFrom<> for HeaderValue
impl From<InvalidHeaderValue> for Error {
    fn from(e: InvalidHeaderValue) -> Self {
        Self::InvalidHeaderValue(e)
    }
}
//Resp::json
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
//Req::query
impl From<serde_qs::Error> for Error {
    fn from(e: serde_qs::Error) -> Self {
        Self::InvalidQueryString(e)
    }
}

#[derive(Debug)]
pub struct Body(Vec<u8>);
impl Body {
    fn empty() -> Self {
        Self(vec![])
    }
}
impl From<String> for Body {
    #[inline]
    fn from(t: String) -> Self {
        Body(t.into_bytes())
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
impl hyper::body::Body for Body {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if self.0.is_empty() {
            std::task::Poll::Ready(None)
        } else {
            let v: Vec<u8> = std::mem::take(self.0.as_mut());
            std::task::Poll::Ready(Some(Ok(Frame::data(v.into()))))
        }
    }
    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.0.len() as u64)
    }
}
